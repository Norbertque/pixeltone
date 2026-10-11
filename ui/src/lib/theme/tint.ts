import { buildScheme, schemeTokens, type PaletteStyle } from './scheme';

/**
 * Inline CSS custom properties that re-theme an element (and its children) with the colour
 * scheme of `seed`, e.g. an album page in its cover's colours.
 */
export function tintStyle(seed: number | null, dark: boolean, style: PaletteStyle, contrast: number): string {
  if (seed === null) return '';
  return Object.entries(schemeTokens(buildScheme(seed, dark, style, contrast)))
    .map(([name, value]) => `${name}: ${value}`)
    .join('; ');
}
