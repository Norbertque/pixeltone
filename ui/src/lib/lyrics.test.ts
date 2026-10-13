import { describe, expect, it } from 'vitest';

import { activeLine, stamp, stampLine } from './lyrics';

const lines = [0, 5000, 9000, 15000].map((timeMs, i) => ({ timeMs, text: `line ${i}` }));

describe('lyrics', () => {
  it('finds the line that is playing', () => {
    expect(activeLine(lines, -1)).toBe(-1);
    expect(activeLine(lines, 0)).toBe(0);
    expect(activeLine(lines, 4999)).toBe(0);
    expect(activeLine(lines, 5000)).toBe(1);
    expect(activeLine(lines, 60_000)).toBe(3);
    expect(activeLine([], 100)).toBe(-1);
  });

  it('writes LRC stamps', () => {
    expect(stamp(83_456)).toBe('[01:23.45]');
    expect(stamp(0)).toBe('[00:00.00]');
    expect(stamp(3_600_000)).toBe('[60:00.00]');
  });

  it('stamps the cursor line and moves on', () => {
    const text = 'first\nsecond\nthird';
    const a = stampLine(text, 0, 1000);
    expect(a.text).toBe('[00:01.00]first\nsecond\nthird');
    expect(a.text.slice(a.cursor)).toBe('second\nthird');
    const b = stampLine(a.text, a.cursor, 2500);
    expect(b.text).toBe('[00:01.00]first\n[00:02.50]second\nthird');
    // Re-stamping replaces the old time.
    const c = stampLine(b.text, 3, 1200);
    expect(c.text.split('\n')[0]).toBe('[00:01.20]first');
    // The last line keeps the cursor at its end.
    const d = stampLine('only', 2, 0);
    expect(d).toEqual({ text: '[00:00.00]only', cursor: 14 });
  });
});
