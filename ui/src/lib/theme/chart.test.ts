import { argbFromHex } from '@material/material-color-utilities';
import { describe, expect, it } from 'vitest';

import { buildScheme, chartMark, PALETTE_STYLES } from './scheme';

/** WCAG relative luminance of an ARGB colour. */
function luminance(argb: number): number {
  const lin = (c: number) => {
    const v = c / 255;
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin((argb >> 16) & 255) + 0.7152 * lin((argb >> 8) & 255) + 0.0722 * lin(argb & 255);
}

function contrast(a: number, b: number): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

/** OKLab lightness (0–1). */
function oklabL(argb: number): number {
  const lin = (c: number) => {
    const v = c / 255;
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  const [r, g, b] = [lin((argb >> 16) & 255), lin((argb >> 8) & 255), lin(argb & 255)];
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
}

const SEEDS = ['#6750a4', '#d4507a', '#1b6b2f', '#ffd166', '#00639b', '#7e57c2', '#808080'].map(argbFromHex);

describe('chart marks', () => {
  it('clear 3:1 against the chart surface and sit in the lightness band, light and dark', () => {
    for (const seed of SEEDS) {
      for (const style of PALETTE_STYLES) {
        for (const dark of [false, true]) {
          const scheme = buildScheme(seed, dark, style);
          const mark = chartMark(scheme);
          const surface = scheme.surfaceContainerLow;
          expect(contrast(mark, surface), `${style} ${dark ? 'dark' : 'light'}`).toBeGreaterThanOrEqual(3);
          const l = oklabL(mark);
          const [lo, hi] = dark ? [0.48, 0.67] : [0.43, 0.77];
          expect(l, `${style} ${dark ? 'dark' : 'light'} L=${l}`).toBeGreaterThanOrEqual(lo);
          expect(l).toBeLessThanOrEqual(hi);
        }
      }
    }
  });
});
