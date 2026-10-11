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
});
