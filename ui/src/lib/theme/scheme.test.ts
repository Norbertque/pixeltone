import { describe, expect, it } from 'vitest';

import { argbFromCss, buildScheme, PALETTE_STYLES, schemeTokens, tokenName } from './scheme';

describe('scheme', () => {
  it('maps colour names to CSS custom properties', () => {
    expect(tokenName('primary_container')).toBe('--md-sys-color-primary-container');
  });

  it('parses CSS hex colours into opaque ARGB', () => {
    expect(argbFromCss('#7E57C2')).toBe(0xff7e57c2);
    expect(argbFromCss('#abc')).toBe(0xffaabbcc);
  });

  it('generates every role for every palette style, light and dark', () => {
    for (const style of PALETTE_STYLES) {
      for (const dark of [false, true]) {
        const tokens = schemeTokens(buildScheme(0xff7e57c2, dark, style));
        expect(tokens['--md-sys-color-primary']).toMatch(/^#[0-9a-f]{6}$/);
        expect(tokens['--md-sys-color-surface-container-high']).toMatch(/^#[0-9a-f]{6}$/);
        expect(tokens['--md-sys-color-on-tertiary-container']).toMatch(/^#[0-9a-f]{6}$/);
      }
    }
  });

  it('dark surfaces are darker than light ones', () => {
    const luminance = (hex: string) => Number.parseInt(hex.slice(1, 3), 16);
    const light = schemeTokens(buildScheme(0xff1e6fd9, false, 'tonalSpot'))['--md-sys-color-surface']!;
    const dark = schemeTokens(buildScheme(0xff1e6fd9, true, 'tonalSpot'))['--md-sys-color-surface']!;
    expect(luminance(dark)).toBeLessThan(luminance(light));
  });
});
