import { describe, expect, it } from 'vitest';

import type { AppSettings, HardwareVolumeInfo, OutputSettings } from './api';
import { DEFAULT_PREFS, formatDb, nudge, volumeControl, volumeIcon } from './volume';

const OUTPUT: OutputSettings = {
  deviceId: 'dac',
  exclusive: false,
  eventDriven: false,
  bufferMs: 50,
  softwareGainDb: -30,
  lineLevel: false,
  softwareVolume: false,
  sharedGainDb: 0,
  fades: true,
  dop: true,
  eq: { enabled: false, preset: 'Flat', gainsDb: Array(10).fill(0) },
  replayGain: 'off',
  replayGainPreampDb: 0,
  crossfadeMs: 0,
  crossfadeAlbums: false,
};
const HW: HardwareVolumeInfo = { db: -33, minDb: -65.25, maxDb: 0, stepDb: 0.75, muted: false };

function settings(output: Partial<OutputSettings> = {}, prefs: Partial<typeof DEFAULT_PREFS> = {}): AppSettings {
  return {
    output: { ...OUTPUT, ...output },
    devices: { dac: { ...DEFAULT_PREFS, ...prefs } },
    preferences: { lyricsOnline: true, artistImagesOnline: true, closeToTray: false, checkUpdates: true },
  };
}

describe('volumeControl', () => {
  it('is the app volume in shared mode, whatever the device has', () => {
    const v = volumeControl(settings({ sharedGainDb: -12 }), HW, null);
    expect(v).toMatchObject({ kind: 'app', db: -12, min: -60, max: 0 });
  });

  it('follows the mode actually in use', () => {
    // Exclusive requested, but the stream fell back to shared.
    expect(volumeControl(settings({ exclusive: true }), HW, 'shared').kind).toBe('app');
    expect(volumeControl(settings({ exclusive: false }), HW, 'exclusive').kind).toBe('device');
  });

  it("uses the device's own volume in exclusive mode when it has one", () => {
    const v = volumeControl(settings({ exclusive: true }), HW, 'exclusive');
    expect(v).toMatchObject({ kind: 'device', db: -33, min: -65.25, max: 0, step: 0.75 });
  });

  it('attenuates in the app where the device has no volume, locked until acknowledged', () => {
    const v = volumeControl(settings({ exclusive: true }, { softwareGainDb: -3 }), null, 'exclusive');
    expect(v).toMatchObject({ kind: 'software', db: -10, max: -10 });
    const unlocked = volumeControl(
      settings({ exclusive: true }, { softwareGainDb: -3, loudnessAcknowledged: true }),
      null,
      'exclusive',
    );
    expect(unlocked).toMatchObject({ db: -3, max: 0 });
  });

  it('lets the app take over from a device volume that is still too loud', () => {
    const v = volumeControl(settings({ exclusive: true }, { softwareVolume: true }), HW, 'exclusive');
    expect(v.kind).toBe('software');
  });

  it('is fixed at line level, which only matters where the app attenuates', () => {
    expect(volumeControl(settings({ exclusive: true }, { lineLevel: true }), null, 'exclusive').kind).toBe('fixed');
    expect(volumeControl(settings({ exclusive: true }, { lineLevel: true }), HW, 'exclusive').kind).toBe('device');
  });

  it('falls back to the default preferences for a device never configured', () => {
    const s = settings({ exclusive: true });
    s.devices = {};
    expect(volumeControl(s, null, null)).toMatchObject({ kind: 'software', db: -30 });
  });
});

describe('formatting and nudging', () => {
  it('formats levels', () => {
    expect(formatDb(-12.25)).toBe('−12.3 dB');
    expect(formatDb(0)).toBe('0 dB');
    expect(formatDb(-60)).toBe('Muted');
    expect(formatDb(-65.25, 'device')).toBe('−65.3 dB');
  });

  it('picks an icon', () => {
    const app = volumeControl(settings({ sharedGainDb: -5 }), null, null);
    expect(volumeIcon(app)).toBe('volume_up');
    expect(volumeIcon({ ...app, db: -45 })).toBe('volume_down');
    expect(volumeIcon({ ...app, db: -60, muted: true })).toBe('volume_off');
  });

  it('nudges in whole decibels on the control grid, within range', () => {
    const app = volumeControl(settings({ sharedGainDb: -10 }), null, null);
    expect(nudge(app, 1)).toBe(-9);
    expect(nudge(app, -3)).toBe(-13);
    expect(nudge({ ...app, db: -0.5 }, 2)).toBe(0);
    const device = volumeControl(settings({ exclusive: true }), HW, 'exclusive');
    expect(nudge(device, 1)).toBeCloseTo(-32.25);
    expect(nudge({ ...device, db: -65 }, -1)).toBe(-65.25);
  });
});
