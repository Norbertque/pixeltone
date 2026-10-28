import { describe, expect, it } from 'vitest';

import { MockQueue } from './mockQueue';

function queue(n: number): MockQueue<string> {
  const q = new MockQueue<string>();
  const slots = Array.from({ length: n }, (_, i) =>
    q.slot({ path: `${i}.flac`, title: String(i), artist: null, album: null, durationMs: 1000, coverKey: null, trackId: i, addedBy: null, guestId: null }, String(i)),
  );
  q.set(slots, 0, false);
  return q;
}

const titles = (q: MockQueue<string>) => q.items.map((s) => s.meta);

describe('mock queue (engine rules)', () => {
  it('play next lands after the current track, add to queue at the end', () => {
    const q = queue(3);
    const [x, y] = ['x', 'y'].map((t) =>
      q.slot({ path: t, title: t, artist: null, album: null, durationMs: 1, coverKey: null, trackId: null, addedBy: null, guestId: null }, t),
    );
    q.add([x!], true);
    q.add([y!], false);
    expect(titles(q)).toEqual(['0', 'x', '1', '2', 'y']);
  });

  it('never removes the playing track and follows it through moves', () => {
    const q = queue(4);
    q.current = 1;
    q.remove(q.items.map((s) => s.entry.id));
    expect(titles(q)).toEqual(['1']);

    const r = queue(4);
    r.current = 2;
    r.move(r.items[2]!.entry.id, 0);
    expect(titles(r)).toEqual(['2', '0', '1', '3']);
    expect(r.current).toBe(0);
  });

  it('shuffle keeps the chosen track first; unshuffle restores the order', () => {
    const q = queue(20);
    q.current = 7;
    q.setShuffle(true);
    expect(q.playing?.meta).toBe('7');
    expect(q.current).toBe(0);
    expect([...titles(q)].sort((a, b) => +a - +b)).toEqual(Array.from({ length: 20 }, (_, i) => String(i)));
    q.setShuffle(false);
    expect(titles(q)).toEqual(Array.from({ length: 20 }, (_, i) => String(i)));
    expect(q.current).toBe(7);
  });

  it('follows repeat rules at the ends', () => {
    const q = queue(3);
    q.current = 2;
    expect(q.after()).toBeNull();
    expect(q.next()).toBeNull();
    q.repeat = 'all';
    expect(q.after()).toBe(0);
    expect(q.next()).toBe(0);
    q.repeat = 'one';
    expect(q.after()).toBe(2);
    expect(q.next()).toBeNull();
    q.current = 0;
    expect(q.previous()).toBe(0);
  });
});
