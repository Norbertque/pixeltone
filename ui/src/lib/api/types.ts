/**
 * Types shared with the Rust core. Core and engine types are generated from Rust by ts-rs
 * (`cargo test` writes `bindings/`); the app-layer types below mirror `src-tauri`.
 */

export type { Conversion } from './bindings/Conversion';
export type { DeviceCapabilities } from './bindings/DeviceCapabilities';
export type { DeviceInfo } from './bindings/DeviceInfo';
export type { FormatCandidate } from './bindings/FormatCandidate';
export type { OutputBadge } from './bindings/OutputBadge';
export type { OutputFormat } from './bindings/OutputFormat';
export type { OutputSettings } from './bindings/OutputSettings';
export type { OutputState } from './bindings/OutputState';
export type { PlaybackStatus } from './bindings/PlaybackStatus';
export type { PlayerSnapshot } from './bindings/PlayerSnapshot';
export type { RepeatMode } from './bindings/RepeatMode';
export type { SampleFormat } from './bindings/SampleFormat';
export type { SampleKind } from './bindings/SampleKind';
export type { ShareMode } from './bindings/ShareMode';
export type { SourceFormat } from './bindings/SourceFormat';
export type { StreamStats } from './bindings/StreamStats';
export type { TrackMeta } from './bindings/TrackMeta';
export type { AlbumDetail } from './bindings/AlbumDetail';
export type { AlbumRow } from './bindings/AlbumRow';
export type { AlbumSort } from './bindings/AlbumSort';
export type { ArtistDetail } from './bindings/ArtistDetail';
export type { ArtistRow } from './bindings/ArtistRow';
export type { FolderEntry } from './bindings/FolderEntry';
export type { FolderListing } from './bindings/FolderListing';
export type { GenreDetail } from './bindings/GenreDetail';
export type { GenreRow } from './bindings/GenreRow';
export type { LibraryFolder } from './bindings/LibraryFolder';
export type { LibraryOverview } from './bindings/LibraryOverview';
export type { ScanPhase } from './bindings/ScanPhase';
export type { ScanProgress } from './bindings/ScanProgress';
export type { SearchResults } from './bindings/SearchResults';
export type { TrackRow } from './bindings/TrackRow';
export type { TrackSort } from './bindings/TrackSort';

import type { DeviceCapabilities } from './bindings/DeviceCapabilities';
import type { DeviceInfo } from './bindings/DeviceInfo';
import type { OutputSettings } from './bindings/OutputSettings';
import type { PlayerSnapshot } from './bindings/PlayerSnapshot';
import type { RepeatMode } from './bindings/RepeatMode';
import type { AlbumDetail } from './bindings/AlbumDetail';
import type { AlbumRow } from './bindings/AlbumRow';
import type { AlbumSort } from './bindings/AlbumSort';
import type { ArtistDetail } from './bindings/ArtistDetail';
import type { ArtistRow } from './bindings/ArtistRow';
import type { FolderListing } from './bindings/FolderListing';
import type { GenreDetail } from './bindings/GenreDetail';
import type { GenreRow } from './bindings/GenreRow';
import type { LibraryFolder } from './bindings/LibraryFolder';
import type { LibraryOverview } from './bindings/LibraryOverview';
import type { ScanProgress } from './bindings/ScanProgress';
import type { SearchResults } from './bindings/SearchResults';
import type { TrackRow } from './bindings/TrackRow';
import type { TrackSort } from './bindings/TrackSort';

// ── App layer (src-tauri/src/commands.rs, settings.rs) ──────────────────────────────

export type PlayerAction = 'play' | 'pause' | 'toggle' | 'stop' | 'next' | 'previous';

export interface DevicePrefs {
  softwareGainDb: number;
  lineLevel: boolean;
  loudnessAcknowledged: boolean;
}
export interface AppSettings {
  output: OutputSettings;
  devices: { [key in string]?: DevicePrefs };
}
export interface HardwareVolumeInfo {
  db: number;
  minDb: number;
  maxDb: number;
  stepDb: number;
  muted: boolean;
}

export interface AppInfo {
  appVersion: string;
  ffmpegVersion: string;
  ffmpegLicense: string;
  os: string;
  outputBackend: string;
}

export interface PlayerError {
  code: string;
  message: string;
}

export type Unlisten = () => void;

export interface Backend {
  appInfo(): Promise<AppInfo>;
  /** Tints the native title bar (Windows 11) to match the theme. */
  setWindowTheme(caption: string, dark: boolean): Promise<void>;

  playerState(): Promise<PlayerSnapshot>;
  playFiles(paths: string[], start: number): Promise<void>;
  playerAction(action: PlayerAction): Promise<void>;
  seek(ms: number): Promise<void>;
  setRepeat(mode: RepeatMode): Promise<void>;
  /** Native file picker for audio files; empty when cancelled. */
  pickAudioFiles(): Promise<string[]>;
  coverUrl(key: string): string;

  outputDevices(): Promise<DeviceInfo[]>;
  outputCapabilities(deviceId: string | null): Promise<DeviceCapabilities>;
  getSettings(): Promise<AppSettings>;
  setOutputSettings(output: OutputSettings, prefs: DevicePrefs): Promise<AppSettings>;
  hardwareVolume(deviceId: string | null): Promise<HardwareVolumeInfo | null>;
  setHardwareVolume(deviceId: string | null, db: number): Promise<void>;

  onPlayerState(cb: (s: PlayerSnapshot) => void): Promise<Unlisten>;
  onPosition(cb: (positionMs: number) => void): Promise<Unlisten>;
  onPlayerError(cb: (e: PlayerError) => void): Promise<Unlisten>;

  // Library
  libraryOverview(): Promise<LibraryOverview>;
  libraryFolders(): Promise<LibraryFolder[]>;
  libraryAddFolder(path: string): Promise<void>;
  libraryRemoveFolder(id: number): Promise<void>;
  libraryRescan(): Promise<void>;
  libraryTracks(sort: TrackSort, offset: number, limit: number): Promise<TrackRow[]>;
  libraryAlbums(sort: AlbumSort, offset: number, limit: number): Promise<AlbumRow[]>;
  libraryAlbum(id: number): Promise<AlbumDetail | null>;
  libraryArtists(offset: number, limit: number): Promise<ArtistRow[]>;
  libraryArtist(id: number): Promise<ArtistDetail | null>;
  libraryGenres(): Promise<GenreRow[]>;
  libraryGenre(id: number): Promise<GenreDetail | null>;
  libraryFolder(path: string | null): Promise<FolderListing>;
  librarySearch(query: string, limit: number): Promise<SearchResults>;
  libraryDelimiters(): Promise<string[]>;
  librarySetDelimiters(delimiters: string[]): Promise<void>;
  /** Plays library tracks, starting at index `start` of `ids`. */
  playTracks(ids: number[], start: number): Promise<void>;
  /** Queues the whole library in `sort` order (or shuffled) and plays from `start`. */
  playLibrary(sort: TrackSort, start: number, shuffled: boolean): Promise<void>;
  /** Native yes/no dialog. */
  confirm(message: string, title: string): Promise<boolean>;
  /** Native folder picker (starts in the Music folder); null when cancelled. */
  pickFolder(): Promise<string | null>;
  /** URL of a library cover thumbnail (300 or 1000 px). */
  artUrl(hash: string, size: 300 | 1000): string;
  onLibraryScan(cb: (p: ScanProgress) => void): Promise<Unlisten>;
  onLibraryChanged(cb: () => void): Promise<Unlisten>;
}

/** File extensions offered in the open dialog (anything FFmpeg decodes also works). */
export const AUDIO_EXTENSIONS = [
  'flac', 'alac', 'm4a', 'mp4', 'wav', 'aif', 'aiff', 'mp3', 'aac', 'ogg', 'oga', 'opus',
  'ape', 'wv', 'wma', 'dsf', 'dff', 'mka', 'tak', 'mpc',
];
