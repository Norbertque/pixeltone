import { describe, expect, it } from 'vitest';

import { PagedRows } from './library.svelte';

/** A fetch over `0..total` that records every request and can be told to fail. */
function source(total: number) {
  const calls: [number, number][] = [];
  let fail = false;
  const fetch = async (offset: number, limit: number): Promise<number[]> => {
    calls.push([offset, limit]);
    if (fail) throw new Error('db locked');
    return Array.from({ length: Math.max(0, Math.min(limit, total - offset)) }, (_, i) => offset + i);
  };
  return { calls, fetch, setFail: (f: boolean) => (fail = f) };
}

const settle = () => new Promise((r) => setTimeout(r, 0));

describe('PagedRows', () => {
  it('starts with holes and loads only the pages a range touches', async () => {
    const s = source(1000);
    const rows = new PagedRows(1000, s.fetch, 100);
    expect(rows.count).toBe(1000);
    expect(rows.rows[0]).toBeUndefined();

    rows.ensure(150, 260);
    await settle();
    expect(s.calls).toEqual([
      [100, 100],
      [200, 100],
    ]);
    expect(rows.rows[100]).toBe(100);
    expect(rows.rows[299]).toBe(299);
    expect(rows.rows[99]).toBeUndefined();
    expect(rows.rows[300]).toBeUndefined();
  });

  it('never fetches a page twice, even while a request is in flight', async () => {
    const s = source(1000);
    const rows = new PagedRows(1000, s.fetch, 100);
    rows.ensure(0, 50);
    rows.ensure(10, 60);
    await settle();
    rows.ensure(0, 100);
    await settle();
    expect(s.calls).toEqual([[0, 100]]);
  });

  it('handles a short last page and empty ranges', async () => {
    const s = source(250);
    const rows = new PagedRows(250, s.fetch, 100);
    rows.ensure(240, 250);
    rows.ensure(5, 5);
    await settle();
    expect(s.calls).toEqual([[200, 100]]);
    expect(rows.rows[249]).toBe(249);
    expect(rows.count).toBe(250);
  });

  it('replaces the array on each load so $state.raw consumers update', async () => {
    const s = source(500);
    const rows = new PagedRows(500, s.fetch, 100);
    const before = rows.rows;
    rows.ensure(0, 10);
    await settle();
    expect(rows.rows).not.toBe(before);
    expect(before[0]).toBeUndefined();
  });

  it('retries a page whose fetch failed', async () => {
    const s = source(300);
    const rows = new PagedRows(300, s.fetch, 100);
    s.setFail(true);
    rows.ensure(0, 10);
    await settle();
    expect(rows.rows[0]).toBeUndefined();

    s.setFail(false);
    rows.ensure(0, 10);
    await settle();
    expect(rows.rows[0]).toBe(0);
    expect(s.calls).toEqual([
      [0, 100],
      [0, 100],
    ]);
  });

  it('refresh keeps old rows until new ones arrive and drops stale answers', async () => {
    let version = 1;
    let release: (() => void) | null = null;
    const fetch = async (offset: number, limit: number): Promise<string[]> => {
      const v = version;
      if (v === 1 && offset === 0 && release === null) {
        // The first answer is slow: it must not overwrite the refreshed data.
        await new Promise<void>((r) => (release = r));
      }
      return Array.from({ length: Math.min(limit, 5 - offset) }, (_, i) => `v${v}-${offset + i}`);
    };
    const rows = new PagedRows(5, fetch, 10);
    rows.ensure(0, 5);
    await settle();
    expect(rows.rows[0]).toBeUndefined();

    version = 2;
    rows.refresh(4);
    await settle();
    expect(rows.count).toBe(4);
    expect(rows.rows[0]).toBe('v2-0');
    release!();
    await settle();
    expect(rows.rows[0]).toBe('v2-0');
    expect(rows.rows).toHaveLength(4);
  });

  it('moves rows locally for an optimistic reorder', async () => {
    const s = source(4);
    const rows = new PagedRows(4, s.fetch, 10);
    rows.ensure(0, 4);
    await settle();
    rows.move(3, 0);
    expect(rows.rows).toEqual([3, 0, 1, 2]);
    rows.move(0, 99);
    expect(rows.rows).toEqual([0, 1, 2, 3]);
  });
});

