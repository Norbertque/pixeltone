import { describe, expect, it } from 'vitest';

import { dailyBars, formatListening, formatTick, hourBars, monthlyBars, niceTicks } from './stats';

const MIN = 60_000;
const HOUR = 60 * MIN;

describe('stats', () => {
  it('formats listening time', () => {
    expect(formatListening(0)).toBe('0 min');
    expect(formatListening(20_000)).toBe('under a minute');
    expect(formatListening(45 * MIN)).toBe('45 min');
    expect(formatListening(3 * HOUR + 12 * MIN)).toBe('3 h 12 min');
    expect(formatListening(2 * HOUR)).toBe('2 h');
    expect(formatTick(30 * MIN)).toBe('30 min');
    expect(formatTick(90 * MIN)).toBe('1.5 h');
  });

  it('picks clean ticks that cover the maximum', () => {
    expect(niceTicks(0)).toEqual([0, 15 * MIN]);
    expect(niceTicks(50 * MIN)).toEqual([0, 15 * MIN, 30 * MIN, 45 * MIN, 60 * MIN]);
    const t = niceTicks(7.5 * HOUR);
    expect(t[0]).toBe(0);
    expect(t.at(-1)!).toBeGreaterThanOrEqual(7.5 * HOUR);
    expect(t.length).toBeLessThanOrEqual(6);
    expect(t[1]! % HOUR).toBe(0);
  });

  it('fills missing days and months with zero', () => {
    const today = new Date(2026, 9, 12);
    const days = dailyBars([{ day: '2026-10-11', ms: 5 }, { day: '2026-09-01', ms: 9 }], 7, today);
    expect(days).toHaveLength(7);
    expect(days.map((d) => d.value)).toEqual([0, 0, 0, 0, 0, 5, 0]);

    const months = monthlyBars([{ day: '2026-10-01', ms: 3 }, { day: '2026-10-09', ms: 4 }, { day: '2026-01-15', ms: 1 }], 12, today);
    expect(months).toHaveLength(12);
    expect(months.at(-1)!.value).toBe(7);
    expect(months[2]!.value).toBe(1); // January 2026: Nov, Dec, Jan…

    const all = monthlyBars([{ day: '2024-10-03', ms: 1 }], null, today);
    expect(all).toHaveLength(25);
    expect(all[0]!.value).toBe(1);
  });

  it('labels every third hour', () => {
    const bars = hourBars(Array.from({ length: 24 }, (_, h) => h));
    expect(bars).toHaveLength(24);
    expect(bars[13]!.title).toBe('13:00–14:00');
    expect(bars.filter((b) => b.label).map((b) => b.label)).toEqual(['0', '3', '6', '9', '12', '15', '18', '21']);
  });
});
