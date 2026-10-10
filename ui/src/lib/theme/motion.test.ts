import { describe, expect, it } from 'vitest';

import { springDuration, springEasing, springPosition, SPRINGS } from './motion';

describe('motion', () => {
  it('springs start at 0 and settle at 1', () => {
    for (const spring of Object.values(SPRINGS)) {
      expect(springPosition(spring, 0)).toBeCloseTo(0, 6);
      expect(springPosition(spring, springDuration(spring) + 0.05)).toBeCloseTo(1, 2);
    }
  });

  it('stiffer springs settle faster', () => {
    expect(springDuration(SPRINGS['spatial-fast'])).toBeLessThan(springDuration(SPRINGS['spatial-default']));
    expect(springDuration(SPRINGS['spatial-default'])).toBeLessThan(springDuration(SPRINGS['spatial-slow']));
  });

  it('spatial springs overshoot, effects springs do not', () => {
    const peak = (s: (typeof SPRINGS)[keyof typeof SPRINGS]) =>
      Math.max(...Array.from({ length: 300 }, (_, i) => springPosition(s, i / 300)));
    expect(peak(SPRINGS['spatial-default'])).toBeGreaterThan(1);
    expect(peak(SPRINGS['effects-default'])).toBeLessThanOrEqual(1);
  });

  it('produces a linear() easing ending at 1', () => {
    const { easing, durationMs } = springEasing(SPRINGS['spatial-default']);
    expect(easing.startsWith('linear(0, ')).toBe(true);
    expect(easing.endsWith(', 1)')).toBe(true);
    expect(durationMs).toBeGreaterThan(150);
    expect(durationMs).toBeLessThan(600);
  });
});
