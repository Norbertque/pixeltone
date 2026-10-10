import { sourceColorFromImageBytes } from '@material/material-color-utilities';

import { api, type OutputBadge, type PlayerSnapshot, type RepeatMode, type TrackMeta } from './api';
import { toasts } from './toasts.svelte';

class Player {
  snapshot = $state<PlayerSnapshot | null>(null);
  /** Last reported position; between updates the UI interpolates with `livePosition()`. */
  positionMs = $state(0);
  /** ARGB seed from the current cover, for theming. */
  artworkSeed = $state<number | null>(null);
  #positionAt = 0;

  readonly status = $derived(this.snapshot?.status ?? 'stopped');
  readonly playing = $derived(this.status === 'playing');
  readonly track = $derived<TrackMeta | null>(this.snapshot?.track ?? null);
  readonly artwork = $derived(this.track?.coverKey ? api.coverUrl(this.track.coverKey) : null);
  readonly durationMs = $derived(this.track?.durationMs ?? 0);
  readonly progress = $derived(this.durationMs > 0 ? Math.min(1, this.positionMs / this.durationMs) : 0);
  readonly badge = $derived<OutputBadge | null>(this.snapshot?.output?.badge ?? null);
  readonly repeat = $derived<RepeatMode>(this.snapshot?.repeat ?? 'off');
  readonly queueLength = $derived(this.snapshot?.queueLength ?? 0);

  setPosition(ms: number): void {
    this.positionMs = Math.max(0, ms);
    this.#positionAt = performance.now();
  }

  /** Position interpolated to "now", for animation frames. Not reactive. */
  livePosition(): number {
    if (!this.playing) return this.positionMs;
    const ms = this.positionMs + (performance.now() - this.#positionAt);
    return this.durationMs > 0 ? Math.min(ms, this.durationMs) : ms;
  }

  apply(snapshot: PlayerSnapshot): void {
    const previousCover = this.track?.coverKey ?? null;
    this.snapshot = snapshot;
    this.setPosition(snapshot.positionMs);
    const cover = snapshot.track?.coverKey ?? null;
    if (cover !== previousCover) this.#updateSeed();
  }

  async #updateSeed(): Promise<void> {
    const url = this.artwork;
    this.artworkSeed = url ? await seedFromImage(url) : null;
  }

  togglePlay(): void {
    if (!this.track) return;
    void api.playerAction('toggle');
  }

  next(): void {
    void api.playerAction('next');
  }

  previous(): void {
    void api.playerAction('previous');
  }

  seek(ms: number): void {
    this.setPosition(ms);
    void api.seek(ms);
  }

  cycleRepeat(): void {
    const next: RepeatMode = this.repeat === 'off' ? 'all' : this.repeat === 'all' ? 'one' : 'off';
    void api.setRepeat(next);
  }

  async openFiles(): Promise<void> {
    const paths = await api.pickAudioFiles();
    if (paths.length) await api.playFiles(paths, 0);
  }
}

export const player = new Player();

/** Subscribes to the core's events and loads the current state. */
export async function initPlayer(): Promise<void> {
  await api.onPlayerState((s) => player.apply(s));
  await api.onPosition((ms) => player.setPosition(ms));
  await api.onPlayerError((e) => toasts.show(e.message, e.code === 'trackFailed' ? 'warning' : 'error'));
  player.apply(await api.playerState());
}

/** "3:58", "1:02:03". */
export function formatTime(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${s}` : `${m}:${s}`;
}

const CODEC_NAMES: Record<string, string> = {
  flac: 'FLAC',
  alac: 'ALAC',
  mp3: 'MP3',
  mp3float: 'MP3',
  aac: 'AAC',
  aac_fixed: 'AAC',
  vorbis: 'Vorbis',
  opus: 'Opus',
  libopus: 'Opus',
  ape: 'APE',
  wavpack: 'WavPack',
  tak: 'TAK',
  wmalossless: 'WMA Lossless',
  wmav2: 'WMA',
  musepack7: 'Musepack',
  musepack8: 'Musepack',
};

export function codecName(codec: string): string {
  if (codec.startsWith('pcm_')) return 'PCM';
  if (codec.startsWith('dsd_')) return 'DSD';
  return CODEC_NAMES[codec] ?? codec.toUpperCase();
}

function rate(hz: number): string {
  return `${Number((hz / 1000).toFixed(1))} kHz`;
}

/** "FLAC · 24-bit/96 kHz", "DSD64", "MP3 · 44.1 kHz". */
export function describeSource(track: TrackMeta): string {
  const s = track.source;
  if (track.dsd) return `DSD${Math.round((s.sampleRate * 8) / 44_100)}`;
  const depth = s.kind === 'int' ? `${s.bits}-bit/` : '';
  return `${codecName(track.codec)} · ${depth}${rate(s.sampleRate)}`;
}

/** Seed colour of an image (downscaled first, the way Android picks wallpaper colours). */
export async function seedFromImage(url: string): Promise<number | null> {
  try {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.decoding = 'async';
    img.src = url;
    await img.decode();
    const size = 112;
    const canvas = new OffscreenCanvas(size, size);
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return null;
    ctx.drawImage(img, 0, 0, size, size);
    const { data } = ctx.getImageData(0, 0, size, size);
    return sourceColorFromImageBytes(data);
  } catch {
    return null;
  }
}
