/**
 * Browser stand-in for the Rust core: a simulated DAC, a demo track and a playback clock,
 * so the UI can be developed (and screenshotted) without Windows.
 */
import { scallopPath } from '../shapes';
import type {
  AppSettings,
  Backend,
  DeviceCapabilities,
  DeviceInfo,
  FormatCandidate,
  HardwareVolumeInfo,
  OutputBadge,
  PlayerError,
  PlayerSnapshot,
  SampleFormat,
  TrackMeta,
} from './types';

const S16: SampleFormat = { kind: 'int', containerBits: 16, validBits: 16 };
const S24_32: SampleFormat = { kind: 'int', containerBits: 32, validBits: 24 };
const S32: SampleFormat = { kind: 'int', containerBits: 32, validBits: 32 };
const F32: SampleFormat = { kind: 'float', containerBits: 32, validBits: 32 };
const HIRES = [44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000, 705_600, 768_000];

const DEVICES: DeviceInfo[] = [
  { id: 'mock-dac', name: 'USB DAC (demo)', isDefault: false },
  { id: 'mock-speakers', name: 'Speakers (demo)', isDefault: true },
];

function caps(device: DeviceInfo): DeviceCapabilities {
  const dac = device.id === 'mock-dac';
  const exclusive: FormatCandidate[] = dac
    ? [S16, S24_32, S32].map((sample) => ({ sample, channels: 2, sampleRates: HIRES }))
    : [S16].map((sample) => ({ sample, channels: 2, sampleRates: [44_100, 48_000, 96_000] }));
  return {
    device,
    mixFormat: { sampleRate: 48_000, channels: 2, sample: F32 },
    exclusive,
    exclusiveError: null,
    hardwareVolume: !dac,
    defaultPeriodUs: 10_000,
    minPeriodUs: 2_667,
  };
}

function demoArtwork(): string {
  const cookie = scallopPath(600, 12, 0.06);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 600">
    <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#3b2a7a"/><stop offset="1" stop-color="#d4507a"/></linearGradient></defs>
    <rect width="600" height="600" fill="url(#g)"/>
    <path d="${cookie}" transform="translate(150 150) scale(0.5)" fill="#ffd166" opacity="0.92"/>
    <circle cx="300" cy="300" r="46" fill="#3b2a7a"/></svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

const DEMO_TRACK: TrackMeta = {
  path: 'C:\\Music\\Aurora Lane\\Pixel Dreams\\03 Neon Tidepool.flac',
  title: 'Neon Tidepool',
  artist: 'Aurora Lane',
  album: 'Pixel Dreams',
  durationMs: 238_000,
  codec: 'flac',
  source: { sampleRate: 96_000, channels: 2, bits: 24, kind: 'int' },
  lossless: true,
  dsd: false,
  coverKey: 'demo',
};

const BADGE: OutputBadge = {
  mode: 'exclusive',
  source: DEMO_TRACK.source,
  output: { sampleRate: 96_000, channels: 2, sample: S24_32 },
  conversion: { resampled: false, depthReduced: false, downmixed: false, dsdToPcm: false },
  gainDb: null,
  bitPerfect: true,
  label: 'Exclusive • 24-bit/96 kHz • Bit-perfect',
};

let settings: AppSettings = {
  output: { deviceId: 'mock-dac', exclusive: true, eventDriven: false, bufferMs: 50, softwareGainDb: -30, lineLevel: false },
  devices: { 'mock-dac': { softwareGainDb: -24, lineLevel: false, loudnessAcknowledged: false } },
};

let snapshot: PlayerSnapshot = {
  status: 'paused',
  track: DEMO_TRACK,
  queueIndex: 0,
  queueLength: 1,
  positionMs: 67_000,
  repeat: 'off',
  output: {
    deviceName: DEVICES[0]!.name,
    requested: 'exclusive',
    active: 'exclusive',
    fallbackReason: null,
    badge: BADGE,
    stats: { underruns: 0, bufferFrames: 4_800, periodFrames: 960 },
  },
};

const stateListeners = new Set<(s: PlayerSnapshot) => void>();
const positionListeners = new Set<(ms: number) => void>();
const errorListeners = new Set<(e: PlayerError) => void>();
let speakersVolume: HardwareVolumeInfo = { db: -12, minDb: -65.25, maxDb: 0, stepDb: 0.75, muted: false };

function publish(next: Partial<PlayerSnapshot>): void {
  snapshot = { ...snapshot, ...next };
  for (const l of stateListeners) l(snapshot);
}

let lastTick = performance.now();
setInterval(() => {
  const now = performance.now();
  const dt = now - lastTick;
  lastTick = now;
  if (snapshot.status !== 'playing') return;
  let pos = snapshot.positionMs + dt;
  if (pos >= (snapshot.track?.durationMs ?? 0)) pos = 0;
  snapshot = { ...snapshot, positionMs: pos };
  for (const l of positionListeners) l(pos);
}, 250);

function subscribe<T>(set: Set<T>, cb: T): Promise<() => void> {
  set.add(cb);
  return Promise.resolve(() => set.delete(cb));
}

export const mockBackend: Backend = {
  appInfo: async () => ({
    appVersion: '0.1.0-dev',
    ffmpegVersion: 'mock (browser preview)',
    ffmpegLicense: 'n/a',
    os: 'browser',
    outputBackend: 'mock',
  }),
  setWindowTheme: async () => {},

  playerState: async () => snapshot,
  playFiles: async () => publish({ status: 'playing', positionMs: 0 }),
  async playerAction(action) {
    switch (action) {
      case 'play':
        return publish({ status: 'playing' });
      case 'pause':
        return publish({ status: 'paused' });
      case 'toggle':
        return publish({ status: snapshot.status === 'playing' ? 'paused' : 'playing' });
      case 'stop':
        return publish({ status: 'stopped', positionMs: 0 });
      case 'next':
      case 'previous':
        return publish({ positionMs: 0 });
    }
  },
  seek: async (ms) => publish({ positionMs: ms }),
  setRepeat: async (repeat) => publish({ repeat }),
  pickAudioFiles: async () => [DEMO_TRACK.path],
  coverUrl: (key) => (key === 'demo' ? demoArtwork() : ''),

  outputDevices: async () => DEVICES,
  async outputCapabilities(deviceId) {
    await new Promise((r) => setTimeout(r, 400)); // probing takes a moment on real hardware
    const device = DEVICES.find((d) => d.id === deviceId) ?? DEVICES.find((d) => d.isDefault)!;
    return caps(device);
  },
  getSettings: async () => structuredClone(settings),
  async setOutputSettings(output, prefs) {
    settings = { output, devices: { ...settings.devices, [output.deviceId ?? '']: prefs } };
    return structuredClone(settings);
  },
  hardwareVolume: async (deviceId) => (deviceId === 'mock-dac' ? null : speakersVolume),
  setHardwareVolume: async (_deviceId, db) => {
    speakersVolume = { ...speakersVolume, db };
  },

  onPlayerState: (cb) => subscribe(stateListeners, cb),
  onPosition: (cb) => subscribe(positionListeners, cb),
  onPlayerError: (cb) => subscribe(errorListeners, cb),
};
