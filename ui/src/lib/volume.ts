/**
 * Which volume control applies to the current output, and how the player bar and Settings
 * show it. Mirrors the engine's rules (crates/pp-engine/src/engine.rs `plan`).
 */
import type { AppSettings, DevicePrefs, HardwareVolumeInfo, ShareMode } from './api';

/** At or below this the app's own volume is muted (pp-core gain.rs). */
export const FLOOR_DB = -60;
/** Highest app level in exclusive mode until the loudness warning is acknowledged. */
export const LOCKED_MAX_DB = -10;
export const DEFAULT_PREFS: DevicePrefs = {
  softwareGainDb: -30,
  lineLevel: false,
  softwareVolume: false,
  loudnessAcknowledged: false,
};

/**
 * - `app`: the app's own volume in shared mode, before Windows mixes.
 * - `device`: the device's volume control; the samples stay untouched.
 * - `software`: the app attenuates in exclusive mode (not bit-perfect below 0 dB).
 * - `fixed`: line level, full scale.
 */
export type VolumeKind = 'app' | 'device' | 'software' | 'fixed';

export interface VolumeControl {
  kind: VolumeKind;
  db: number;
  min: number;
  max: number;
  step: number;
  muted: boolean;
  /** Names the control, e.g. for the slider's accessible label. */
  label: string;
}

export function devicePrefs(settings: AppSettings): DevicePrefs {
  return { ...DEFAULT_PREFS, ...settings.devices[settings.output.deviceId ?? ''] };
}

/**
 * The control for `settings`. `active` is the mode actually in use (exclusive mode can
 * fall back to shared); without a stream, the requested one counts.
 */
export function volumeControl(
  settings: AppSettings,
  hardware: HardwareVolumeInfo | null,
  active: ShareMode | null,
): VolumeControl {
  const out = settings.output;
  const mode = active ?? (out.exclusive ? 'exclusive' : 'shared');
  if (mode === 'shared') {
    const db = Math.min(0, out.sharedGainDb);
    return { kind: 'app', db, min: FLOOR_DB, max: 0, step: 0.5, muted: db <= FLOOR_DB, label: 'App volume' };
  }
  const prefs = devicePrefs(settings);
  if (hardware && !prefs.softwareVolume) {
    return {
      kind: 'device',
      db: hardware.db,
      min: hardware.minDb,
      max: hardware.maxDb,
      step: hardware.stepDb > 0 ? hardware.stepDb : 0.5,
      muted: hardware.muted,
      label: 'Device volume',
    };
  }
  if (prefs.lineLevel) {
    return { kind: 'fixed', db: 0, min: 0, max: 0, step: 1, muted: false, label: 'Line level' };
  }
  const max = prefs.loudnessAcknowledged ? 0 : LOCKED_MAX_DB;
  const db = Math.min(prefs.softwareGainDb, max);
  return { kind: 'software', db, min: FLOOR_DB, max, step: 0.5, muted: db <= FLOOR_DB, label: 'App volume' };
}

/** "−12.5 dB", or "Muted" at the bottom of the app's own range. */
export function formatDb(db: number, kind: VolumeKind = 'app'): string {
  if (kind !== 'device' && db <= FLOOR_DB) return 'Muted';
  if (Math.abs(db) < 0.05) return '0 dB';
  return `${db > 0 ? '+' : '−'}${Math.abs(db).toFixed(1)} dB`;
}

export function volumeIcon(v: VolumeControl): string {
  if (v.muted) return 'volume_off';
  if (v.kind === 'fixed') return 'volume_up';
  return v.db - v.min < (v.max - v.min) / 2 ? 'volume_down' : 'volume_up';
}

/** A level `steps` notches away (mouse wheel, arrow keys), on the control's grid. */
export function nudge(v: VolumeControl, steps: number): number {
  const notch = Math.max(v.step, 1);
  const raw = v.db + steps * notch;
  const snapped = Math.round(raw / v.step) * v.step;
  return Math.min(v.max, Math.max(v.min, snapped));
}
