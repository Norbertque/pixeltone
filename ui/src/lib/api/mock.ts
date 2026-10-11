/**
 * Browser stand-in for the Rust core: a simulated DAC, a demo track and a playback clock,
 * so the UI can be developed (and screenshotted) without Windows.
 */
import { scallopPath } from '../shapes';
import { mockArt, mockLibrary } from './mockLibrary';
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
  ScanProgress,
  TrackMeta,
  TrackRow,
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

const scanListeners = new Set<(p: ScanProgress) => void>();
const changedListeners = new Set<() => void>();
let scanning = false;
let delimiters = [';', '/', ' feat. ', ' ft. ', ' featuring ', ' feat ', ' ft '];

function metaFromRow(t: TrackRow): TrackMeta {
  return {
    path: t.path,
    title: t.title,
    artist: t.artist,
    album: t.album,
    durationMs: t.durationMs,
    codec: t.codec,
    source: {
      sampleRate: t.dsd ? t.sampleRate / 8 : t.sampleRate,
      channels: 2,
      bits: t.bits || 32,
      kind: t.lossless && !t.dsd ? 'int' : 'float',
    },
    lossless: t.lossless,
    dsd: t.dsd,
    coverKey: t.cover ? `lib/${t.cover}` : null,
  };
}

function simulateScan(): void {
  if (scanning) return;
  scanning = true;
  const total = 240;
  let done = 0;
  const tick = setInterval(() => {
    done = Math.min(total, done + 24);
    const finished = done >= total;
    const progress: ScanProgress = {
      phase: finished ? 'done' : 'reading',
      done,
      total,
      added: Math.floor(done / 10),
      updated: 0,
      removed: 0,
      errors: 0,
    };
    for (const l of scanListeners) l(progress);
    if (finished) {
      clearInterval(tick);
      scanning = false;
      for (const l of changedListeners) l();
    }
  }, 200);
}

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
  coverUrl: (key) => (key === 'demo' ? demoArtwork() : key.startsWith('lib/') ? mockArt(key.slice(4)) : ''),

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

  libraryOverview: async () => mockLibrary.overview(scanning),
  libraryFolders: async () => mockLibrary.folders(),
  libraryAddFolder: async () => simulateScan(),
  libraryRemoveFolder: async () => {},
  libraryRescan: async () => simulateScan(),
  libraryTracks: async (sort, offset, limit) => mockLibrary.tracks(sort, offset, limit),
  libraryAlbums: async (sort, offset, limit) => mockLibrary.albums(sort, offset, limit),
  libraryAlbum: async (id) => mockLibrary.album(id),
  libraryArtists: async (offset, limit) => mockLibrary.artists(offset, limit),
  libraryArtist: async (id) => mockLibrary.artist(id),
  libraryGenres: async () => mockLibrary.genres(),
  libraryGenre: async (id) => mockLibrary.genre(id),
  libraryFolder: async (path) => mockLibrary.folder(path),
  librarySearch: async (query, limit) => mockLibrary.search(query, limit),
  libraryDelimiters: async () => [...delimiters],
  librarySetDelimiters: async (d) => {
    delimiters = [...d];
    simulateScan();
  },
  async playTracks(ids, start) {
    const row = mockLibrary.track(ids[start] ?? ids[0] ?? -1);
    if (!row) return;
    publish({
      status: 'playing',
      track: metaFromRow(row),
      positionMs: 0,
      queueIndex: start,
      queueLength: ids.length,
    });
  },
  async playLibrary(sort, start, shuffled) {
    const rows = mockLibrary.tracks(sort, 0, 100_000);
    const index = shuffled ? Math.floor(Math.random() * rows.length) : start;
    const row = rows[index];
    if (row) {
      publish({ status: 'playing', track: metaFromRow(row), positionMs: 0, queueIndex: index, queueLength: rows.length });
    }
  },
  confirm: async (message) => window.confirm(message),
  pickFolder: async () => 'C:\\Users\\You\\Music',
  artUrl: (hash) => mockArt(hash),
  onLibraryScan: (cb) => subscribe(scanListeners, cb),
  onLibraryChanged: (cb) => subscribe(changedListeners, cb),
};
