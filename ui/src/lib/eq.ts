/** The equaliser's bands and built-in curves (the core runs whatever gains it is given). */
import type { EqSettings } from './api';

/** Band centres in Hz (crates/pp-core/src/eq.rs `BANDS`). */
export const BANDS = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000] as const;
/** Band gain limit, dB. */
export const MAX_DB = 12;

export const PRESETS: readonly { name: string; gains: readonly number[] }[] = [
  { name: 'Flat', gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
  { name: 'Bass boost', gains: [6, 5, 4, 2, 0.5, 0, 0, 0, 0, 0] },
  { name: 'Treble boost', gains: [0, 0, 0, 0, 0, 0.5, 2, 3.5, 5, 6] },
  { name: 'Vocal', gains: [-2, -2, -1, 1, 3, 3.5, 2.5, 1, 0, -1] },
  { name: 'Rock', gains: [4, 3, 1.5, -0.5, -1, 0, 1.5, 3, 3.5, 4] },
  { name: 'Pop', gains: [-1, 1, 2.5, 3.5, 3, 0.5, -0.5, -1, 0, 0.5] },
  { name: 'Jazz', gains: [3, 2, 1, 1.5, -1, -1, 0, 1, 2, 3] },
  { name: 'Classical', gains: [3, 2, 1, 0, 0, 0, -0.5, -0.5, 1, 2] },
  { name: 'Electronic', gains: [5, 4, 1.5, 0, -1.5, 1, 0, 1, 3.5, 4.5] },
  { name: 'Loudness', gains: [5, 3, 0.5, 0, -0.5, 0, 0, 0.5, 2.5, 4] },
];
export const CUSTOM = 'Custom';

export function bandLabel(hz: number): string {
  return hz >= 1000 ? `${hz / 1000}k` : String(hz);
}

/** The curve of a preset (the settings keep its name). */
export function applyPreset(eq: EqSettings, name: string): EqSettings {
  const preset = PRESETS.find((p) => p.name === name);
  return preset ? { ...eq, preset: name, gainsDb: [...preset.gains] } : eq;
}

/** One band moved: the curve is now the user's own, unless it matches a preset again. */
export function setBand(eq: EqSettings, band: number, db: number): EqSettings {
  const gainsDb = BANDS.map((_, i) => (i === band ? clampDb(db) : (eq.gainsDb[i] ?? 0)));
  const match = PRESETS.find((p) => p.gains.every((g, i) => Math.abs(g - gainsDb[i]!) < 0.05));
  return { ...eq, gainsDb, preset: match?.name ?? CUSTOM };
}

function clampDb(db: number): number {
  return Math.max(-MAX_DB, Math.min(MAX_DB, Math.round(db * 2) / 2));
}
