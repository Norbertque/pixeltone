import { describe, expect, it } from 'vitest';

import { clampTop, displacement, dropIndex, edgeScroll } from './drag';

describe('drag to reorder', () => {
  it('lands on the nearest slot and never outside the list', () => {
    expect(dropIndex(0, 64, 10)).toBe(0);
    expect(dropIndex(31, 64, 10)).toBe(0);
    expect(dropIndex(33, 64, 10)).toBe(1);
    expect(dropIndex(64 * 20, 64, 10)).toBe(9);
    expect(dropIndex(-50, 64, 10)).toBe(0);
    expect(dropIndex(100, 64, 0)).toBe(0);
    expect(clampTop(-10, 64, 10)).toBe(0);
    expect(clampTop(10_000, 64, 10)).toBe(9 * 64);
  });

  it('slides the rows between the old and the new place', () => {
    // Row 1 dragged down to 3: rows 2 and 3 move up.
    expect([0, 1, 2, 3, 4].map((i) => displacement(i, 1, 3, 64))).toEqual([0, 0, -64, -64, 0]);
    // Row 3 dragged up to 1: rows 1 and 2 move down.
    expect([0, 1, 2, 3, 4].map((i) => displacement(i, 3, 1, 64))).toEqual([0, 64, 64, 0, 0]);
    expect([0, 1, 2].map((i) => displacement(i, 1, 1, 64))).toEqual([0, 0, 0]);
  });

  it('scrolls faster the closer the pointer is to an edge', () => {
    expect(edgeScroll(300, 0, 600)).toBe(0);
    expect(edgeScroll(0, 0, 600)).toBe(-18);
    expect(edgeScroll(40, 0, 600)).toBeLessThan(0);
    expect(edgeScroll(40, 0, 600)).toBeGreaterThan(-18);
    expect(edgeScroll(600, 0, 600)).toBe(18);
  });
});
