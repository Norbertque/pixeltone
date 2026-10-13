import {
  Hct,
  SchemeContent,
  SchemeExpressive,
  SchemeFidelity,
  SchemeFruitSalad,
  SchemeMonochrome,
  SchemeNeutral,
  SchemeRainbow,
  SchemeTonalSpot,
  SchemeVibrant,
  hexFromArgb,
  type DynamicScheme,
} from '@material/material-color-utilities';

/** Material You palette styles, as offered on Android. */
export const PALETTE_STYLES = [
  'tonalSpot',
  'expressive',
  'vibrant',
  'fidelity',
  'content',
  'rainbow',
  'fruitSalad',
  'neutral',
  'monochrome',
] as const;
export type PaletteStyle = (typeof PALETTE_STYLES)[number];

export const PALETTE_STYLE_LABELS: Record<PaletteStyle, string> = {
  tonalSpot: 'Tonal spot',
  expressive: 'Expressive',
  vibrant: 'Vibrant',
  fidelity: 'Fidelity',
  content: 'Content',
  rainbow: 'Rainbow',
  fruitSalad: 'Fruit salad',
  neutral: 'Neutral',
  monochrome: 'Monochrome',
};

type SchemeCtor = new (source: Hct, isDark: boolean, contrast: number, spec?: '2021' | '2025') => DynamicScheme;

const SCHEMES: Record<PaletteStyle, SchemeCtor> = {
  tonalSpot: SchemeTonalSpot,
  expressive: SchemeExpressive,
  vibrant: SchemeVibrant,
  fidelity: SchemeFidelity,
  content: SchemeContent,
  rainbow: SchemeRainbow,
  fruitSalad: SchemeFruitSalad,
  neutral: SchemeNeutral,
  monochrome: SchemeMonochrome,
};

/** Builds an M3 scheme with the 2025 (Expressive) colour spec where the style supports it. */
export function buildScheme(seedArgb: number, dark: boolean, style: PaletteStyle, contrast = 0): DynamicScheme {
  const Ctor = SCHEMES[style];
  return new Ctor(Hct.fromInt(seedArgb), dark, contrast, '2025');
}

/** `primary_container` → `--md-sys-color-primary-container`. */
export function tokenName(colorName: string): string {
  return `--md-sys-color-${colorName.replaceAll('_', '-')}`;
}

/**
 * Data marks (chart bars) from the primary palette: tone 40 in light, 58 in dark. M3's dark
 * `primary` (tone 80) is too light and pale for marks on a dark surface.
 */
export function chartMark(scheme: DynamicScheme): number {
  return scheme.primaryPalette.tone(scheme.isDark ? 58 : 40);
}

/** Every colour role of the scheme as CSS custom properties. */
export function schemeTokens(scheme: DynamicScheme): Record<string, string> {
  const tokens: Record<string, string> = {};
  for (const color of scheme.colors.allColors) {
    tokens[tokenName(color.name)] = hexFromArgb(color.getArgb(scheme));
  }
  tokens['--pp-chart-mark'] = hexFromArgb(chartMark(scheme));
  return tokens;
}

export function argbFromCss(hex: string): number {
  const clean = hex.replace('#', '');
  const full = clean.length === 3 ? [...clean].map((c) => c + c).join('') : clean;
  return (0xff000000 | Number.parseInt(full, 16)) >>> 0;
}
