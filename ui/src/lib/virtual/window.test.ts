import { describe, expect, it } from 'vitest';

import { gridColumns, visibleRange } from './window';

describe('visibleRange', () => {
  it('covers the viewport plus overscan', () => {
    expect(visibleRange(0, 560, 56, 1000, 4)).toEqual([0, 15]);
    // Scrolled 10 rows past the list's top.
    expect(visibleRange(-560, 560, 56, 1000, 4)).toEqual([6, 25]);
  });

  it('clamps to the list', () => {
    expect(visibleRange(-100_000, 560, 56, 1000, 4)).toEqual([1000, 1000]);
    expect(visibleRange(-55_440, 560, 56, 1000, 4)).toEqual([986, 1000]);
    // List starts 400 px down: ~3 rows visible, plus overscan.
    expect(visibleRange(400, 560, 56, 10, 4)).toEqual([0, 7]);
    expect(visibleRange(0, 560, 56, 0, 4)).toEqual([0, 0]);
  });
});

describe('gridColumns', () => {
  it('fits as many columns as allowed', () => {
    expect(gridColumns(1000, 180, 20)).toEqual({ cols: 5, itemWidth: 184 });
    expect(gridColumns(150, 180, 20)).toEqual({ cols: 1, itemWidth: 150 });
  });
});
