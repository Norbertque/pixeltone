/**
 * Types shared with the Rust core. Core and engine types are generated from Rust by ts-rs
 * (`cargo test` writes `bindings/`); the app-layer types below mirror `src-tauri`.
 */

export type { Conversion } from './bindings/Conversion';
export type { DeviceCapabilities } from './bindings/DeviceCapabilities';
export type { DeviceChange } from './bindings/DeviceChange';
export type { DeviceInfo } from './bindings/DeviceInfo';
export type { Effects } from './bindings/Effects';
export type { EqSettings } from './bindings/EqSettings';
export type { FormatCandidate } from './bindings/FormatCandidate';
export type { OutputBadge } from './bindings/OutputBadge';
export type { OutputFormat } from './bindings/OutputFormat';
export type { OutputSettings } from './bindings/OutputSettings';
export type { OutputState } from './bindings/OutputState';
export type { PlaybackStatus } from './bindings/PlaybackStatus';
export type { PlayerSnapshot } from './bindings/PlayerSnapshot';
export type { QueueEntry } from './bindings/QueueEntry';
export type { RepeatMode } from './bindings/RepeatMode';
export type { ReplayGainMode } from './bindings/ReplayGainMode';
export type { SampleFormat } from './bindings/SampleFormat';
export type { SampleKind } from './bindings/SampleKind';
export type { ShareMode } from './bindings/ShareMode';
export type { SourceFormat } from './bindings/SourceFormat';
export type { TagFields } from './bindings/TagFields';
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
export type { DayListening } from './bindings/DayListening';
export type { HomeFeed } from './bindings/HomeFeed';
export type { ListeningStats } from './bindings/ListeningStats';
export type { LyricLine } from './bindings/LyricLine';
export type { Lyrics } from './bindings/Lyrics';
export type { LyricsSource } from './bindings/LyricsSource';
export type { PlaylistDetail } from './bindings/PlaylistDetail';
export type { PlaylistInput } from './bindings/PlaylistInput';
export type { PlaylistRow } from './bindings/PlaylistRow';
export type { TopAlbum } from './bindings/TopAlbum';
export type { TopArtist } from './bindings/TopArtist';
export type { TopTrack } from './bindings/TopTrack';

import type { DeviceCapabilities } from './bindings/DeviceCapabilities';
import type { DeviceChange } from './bindings/DeviceChange';
import type { DeviceInfo } from './bindings/DeviceInfo';
import type { OutputSettings } from './bindings/OutputSettings';
import type { PlayerSnapshot } from './bindings/PlayerSnapshot';
import type { QueueEntry } from './bindings/QueueEntry';
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
import type { HomeFeed } from './bindings/HomeFeed';
import type { ListeningStats } from './bindings/ListeningStats';
import type { Lyrics } from './bindings/Lyrics';
import type { PlaylistDetail } from './bindings/PlaylistDetail';
import type { PlaylistRow } from './bindings/PlaylistRow';
import type { TagFields } from './bindings/TagFields';

// ── App layer (src-tauri/src/commands.rs, settings.rs) ──────────────────────────────

export type PlayerAction = 'play' | 'pause' | 'toggle' | 'stop' | 'next' | 'previous';

export interface DevicePrefs {
  softwareGainDb: number;
  lineLevel: boolean;
  /** Attenuate in the app although the device has its own volume control. */
  softwareVolume: boolean;
  loudnessAcknowledged: boolean;
}
export interface Preferences {
  /** Look lyrics up on LRCLIB when a track has none of its own. */
  lyricsOnline: boolean;
  /** Download artist pictures from Deezer. */
  artistImagesOnline: boolean;
  /** Closing the main window hides it; playback goes on, controlled from the tray. */
  closeToTray: boolean;
  /** Look for a new version at start-up. */
  checkUpdates: boolean;
  /** Phones on the same network can find, pair with and control the player. */
  remoteEnabled: boolean;
}
/** Remote control (src-tauri/src/remote.rs). */
export interface RemoteDevice {
  id: string;
  name: string;
  /** Unix seconds. */
  pairedAt: number;
  lastSeen: number;
}
export interface RemoteStatus {
  enabled: boolean;
  running: boolean;
  /** How phones list this PC. */
  name: string;
  /** `ip:port`, for phones that can't find the PC by themselves. */
  address: string | null;
  /** The pairing code on show, and for how many more seconds. */
  code: string | null;
  codeSeconds: number;
  devices: RemoteDevice[];
}
/** A jam (crates/pp-remote, `JamView`): a shared queue that guests join with its QR code. */
export interface GuestPermissions {
  addTracks: boolean;
  /** Remove and move the tracks they added. */
  changeOwnTracks: boolean;
  /** Play, pause, skip and seek. */
  controlPlayback: boolean;
}
export interface JamGuest {
  id: string;
  name: string;
}
/** A removed guest asking to come back. */
export interface RejoinRequest {
  id: string;
  name: string;
}
export interface JamView {
  /** What the QR code shows. */
  link: string;
  permissions: GuestPermissions;
  guests: JamGuest[];
  requests: RejoinRequest[];
}
/** A newer version, from the update feed. */
export interface UpdateInfo {
  version: string;
  notes: string | null;
}
export interface AppSettings {
  output: OutputSettings;
  devices: { [key in string]?: DevicePrefs };
  preferences: Preferences;
}

/** The track lyrics are wanted for (src-tauri/src/userdata.rs). */
export interface LyricsRequest {
  path: string;
  title: string;
  artist: string | null;
  album: string | null;
  durationMs: number | null;
}
export interface HardwareVolumeInfo {
  db: number;
  minDb: number;
  maxDb: number;
  stepDb: number;
  muted: boolean;
}

/** AI playlists (src-tauri/src/ai.rs). */
export interface AiStatus {
  hasKey: boolean;
  /** Songs in the library. */
  songs: number;
  /** Songs sent with a request (big libraries are sampled). */
  sentSongs: number;
  /** Rough price of one request, US dollars. */
  estimateUsd: number;
}
export interface AiPlaylistResult {
  playlistId: number;
  name: string;
  description: string;
  songs: number;
}

/** What writing tags did (src-tauri/src/tags.rs). */
export interface TagWriteReport {
  written: number;
  /** "file name: reason" for each file that couldn't be written. */
  failed: string[];
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
  /** Plays files in the given order from `start` (shuffle off). */
  playFiles(paths: string[], start: number): Promise<void>;
  playerAction(action: PlayerAction): Promise<void>;
  seek(ms: number): Promise<void>;
  setRepeat(mode: RepeatMode): Promise<void>;
  setShuffle(on: boolean): Promise<void>;

  /** Queue rows in play order. */
  queueItems(offset: number, limit: number): Promise<QueueEntry[]>;
  /** Removes queue items (never the one playing). */
  queueRemove(ids: number[]): Promise<void>;
  /** Moves a queue item to `to` in play order. */
  queueMove(id: number, to: number): Promise<void>;
  /** Removes everything but the current track. */
  queueClear(): Promise<void>;
  /** Plays a queue item. */
  queueJump(id: number): Promise<void>;
  /** "Play next" (`next`) or "Add to queue" for library tracks. */
  queueAddTracks(ids: number[], next: boolean): Promise<void>;
  /** "Play next" (`next`) or "Add to queue" for files. */
  queueAddFiles(paths: string[], next: boolean): Promise<void>;
  /** Native file picker for audio files; empty when cancelled. */
  pickAudioFiles(): Promise<string[]>;
  coverUrl(key: string): string;

  outputDevices(): Promise<DeviceInfo[]>;
  outputCapabilities(deviceId: string | null): Promise<DeviceCapabilities>;
  getSettings(): Promise<AppSettings>;
  setOutputSettings(output: OutputSettings, prefs: DevicePrefs): Promise<AppSettings>;
  hardwareVolume(deviceId: string | null): Promise<HardwareVolumeInfo | null>;
  setHardwareVolume(deviceId: string | null, db: number): Promise<void>;
  setHardwareMute(deviceId: string | null, muted: boolean): Promise<void>;
  /** Settings changed (from any window or command). */
  onSettingsChanged(cb: (s: AppSettings) => void): Promise<Unlisten>;

  onPlayerState(cb: (s: PlayerSnapshot) => void): Promise<Unlisten>;
  onPosition(cb: (positionMs: number) => void): Promise<Unlisten>;
  onPlayerError(cb: (e: PlayerError) => void): Promise<Unlisten>;
  /** Audio devices were added, removed or changed roles. */
  onDevicesChanged(cb: (c: DeviceChange) => void): Promise<Unlisten>;

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
  /**
   * Plays library tracks from index `start` of `ids` (`null`: the top, or a random track when
   * shuffling). `shuffle` turns shuffle on or off; `null` keeps the current mode.
   */
  playTracks(ids: number[], start: number | null, shuffle: boolean | null): Promise<void>;
  /** Queues the whole library in `sort` order and plays it, like `playTracks`. */
  playLibrary(sort: TrackSort, start: number | null, shuffle: boolean | null): Promise<void>;
  /** Native yes/no dialog. */
  confirm(message: string, title: string): Promise<boolean>;
  /** Native folder picker (starts in the Music folder); null when cancelled. */
  pickFolder(): Promise<string | null>;
  /** URL of a library cover thumbnail (300 or 1000 px). */
  artUrl(hash: string, size: 300 | 1000): string;
  onLibraryScan(cb: (p: ScanProgress) => void): Promise<Unlisten>;
  onLibraryChanged(cb: () => void): Promise<Unlisten>;

  setPreferences(preferences: Preferences): Promise<AppSettings>;

  // Playlists
  libraryPlaylists(): Promise<PlaylistRow[]>;
  libraryPlaylist(id: number): Promise<PlaylistDetail | null>;
  /** Creates a playlist holding `trackIds`; returns its id. */
  playlistCreate(name: string, trackIds: number[]): Promise<number>;
  playlistRename(id: number, name: string): Promise<void>;
  playlistDelete(id: number): Promise<void>;
  playlistAddTracks(id: number, trackIds: number[]): Promise<void>;
  /** Removes the entries at `positions` (0-based). */
  playlistRemove(id: number, positions: number[]): Promise<void>;
  playlistMove(id: number, from: number, to: number): Promise<void>;
  /** Imports an M3U/M3U8 file as a new playlist; returns its id. */
  playlistImport(path: string): Promise<number>;
  playlistExport(id: number, path: string): Promise<void>;
  /** Plays a playlist like `playTracks`. */
  playPlaylist(id: number, start: number | null, shuffle: boolean | null): Promise<void>;
  /** Native open dialog for .m3u/.m3u8 files; null when cancelled. */
  pickPlaylistFile(): Promise<string | null>;
  /** Native save dialog for exporting `name`; null when cancelled. */
  pickPlaylistSaveFile(name: string): Promise<string | null>;

  // Favourites, history, Home
  librarySetFavorite(path: string, on: boolean): Promise<void>;
  libraryIsFavorite(path: string): Promise<boolean>;
  libraryFavorites(): Promise<TrackRow[]>;
  /** Listening over the last `days` days, or all time. */
  libraryStats(days: number | null): Promise<ListeningStats>;
  libraryHome(): Promise<HomeFeed>;
  /** Plays today's mix from track `start`. */
  playDailyMix(start: number | null): Promise<void>;

  // Lyrics
  /** Lyrics from the file, a saved copy, embedded tags or LRCLIB; null when there are none. */
  lyricsFind(track: LyricsRequest): Promise<Lyrics | null>;
  /** Saves lyrics (LRC or plain text) next to the track, or inside the app. */
  lyricsSave(track: LyricsRequest, text: string): Promise<Lyrics>;

  // Remote control
  remoteStatus(): Promise<RemoteStatus>;
  /** Shows a new pairing code. */
  remotePair(): Promise<RemoteStatus>;
  remoteStopPairing(): Promise<RemoteStatus>;
  remoteForget(id: string): Promise<RemoteStatus>;

  // Jam
  jamState(): Promise<JamView | null>;
  /** Starts a jam; the remote control server runs for it even when turned off. */
  jamStart(): Promise<JamView>;
  /** Ends it: guests are out, and the tracks they sent from their phones leave the queue. */
  jamEnd(): Promise<void>;
  jamPermissions(permissions: GuestPermissions): Promise<void>;
  /** Takes a guest out, with their tracks still to come. */
  jamRemoveGuest(guestId: string): Promise<void>;
  /** Answers a rejoin request; false when it isn't waiting any more. */
  jamAnswer(requestId: string, allow: boolean): Promise<boolean>;
  onJam(cb: (jam: JamView | null) => void): Promise<Unlisten>;

  // Updates
  /** A newer version than this one, or null. */
  checkForUpdate(): Promise<UpdateInfo | null>;
  /** Downloads and installs the update found by `checkForUpdate`, then restarts. */
  installUpdate(onProgress: (fraction: number | null) => void): Promise<void>;

  // Windows
  /** Swaps the main window for the small always-on-top player. */
  openMiniPlayer(): Promise<void>;
  /** Brings the main window back (and closes the mini player). */
  showMainWindow(): Promise<void>;

  // AI playlists (the key goes to Windows Credential Manager and never comes back)
  aiStatus(): Promise<AiStatus>;
  aiSetKey(key: string): Promise<void>;
  aiRemoveKey(): Promise<void>;
  /** Asks Claude for a playlist of up to `songs` songs and saves it. */
  aiPlaylist(prompt: string, songs: number): Promise<AiPlaylistResult>;

  // Tag editor
  /** The editable fields of each file, in order. */
  tagsRead(paths: string[]): Promise<TagFields[]>;
  /** Writes `change` (null fields untouched, "" removes) into every file, then rescans. */
  tagsWrite(paths: string[], change: TagFields): Promise<TagWriteReport>;
}

/** File extensions offered in the open dialog (anything FFmpeg decodes also works). */
export const AUDIO_EXTENSIONS = [
  'flac', 'alac', 'm4a', 'mp4', 'wav', 'aif', 'aiff', 'mp3', 'aac', 'ogg', 'oga', 'opus',
  'ape', 'wv', 'wma', 'dsf', 'dff', 'mka', 'tak', 'mpc',
];
