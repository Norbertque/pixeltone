import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { audioDir, join } from '@tauri-apps/api/path';
import { ask, open, save } from '@tauri-apps/plugin-dialog';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';

import {
  AUDIO_EXTENSIONS,
  type AppInfo,
  type AlbumDetail,
  type AlbumRow,
  type AppSettings,
  type ArtistDetail,
  type ArtistRow,
  type Backend,
  type FolderListing,
  type GenreDetail,
  type GenreRow,
  type LibraryFolder,
  type LibraryOverview,
  type ScanProgress,
  type SearchResults,
  type TrackRow,
  type DeviceCapabilities,
  type DeviceChange,
  type DeviceInfo,
  type HardwareVolumeInfo,
  type PlayerError,
  type PlayerSnapshot,
  type QueueEntry,
  type HomeFeed,
  type ListeningStats,
  type Lyrics,
  type PlaylistDetail,
  type PlaylistRow,
  type TagFields,
  type TagWriteReport,
  type AiStatus,
  type AiPlaylistResult,
  type UpdateInfo,
} from './types';

let pendingUpdate: Update | null = null;

export const tauriBackend: Backend = {
  appInfo: () => invoke<AppInfo>('app_info'),
  setWindowTheme: (caption, dark) => invoke('set_window_theme', { caption, dark }),

  playerState: () => invoke<PlayerSnapshot>('player_state'),
  playFiles: (paths, start) => invoke('player_play_files', { paths, start }),
  playerAction: (action) => invoke('player_action', { action }),
  seek: (ms) => invoke('player_seek', { ms }),
  setRepeat: (mode) => invoke('player_set_repeat', { mode }),
  setShuffle: (on) => invoke('player_set_shuffle', { on }),

  queueItems: (offset, limit) => invoke<QueueEntry[]>('queue_items', { offset, limit }),
  queueRemove: (ids) => invoke('queue_remove', { ids }),
  queueMove: (id, to) => invoke('queue_move', { id, to }),
  queueClear: () => invoke('queue_clear'),
  queueJump: (id) => invoke('queue_jump', { id }),
  queueAddTracks: (ids, next) => invoke('queue_add_tracks', { ids, next }),
  queueAddFiles: (paths, next) => invoke('queue_add_files', { paths, next }),
  async pickAudioFiles() {
    const picked = await open({
      multiple: true,
      directory: false,
      title: 'Play files',
      filters: [{ name: 'Audio', extensions: AUDIO_EXTENSIONS }],
    });
    if (!picked) return [];
    return Array.isArray(picked) ? picked : [picked];
  },
  coverUrl: (key) => convertFileSrc(key, 'cover'),

  outputDevices: () => invoke<DeviceInfo[]>('output_devices'),
  outputCapabilities: (deviceId) => invoke<DeviceCapabilities>('output_capabilities', { deviceId }),
  getSettings: () => invoke<AppSettings>('get_settings'),
  setOutputSettings: (output, prefs) => invoke<AppSettings>('set_output_settings', { output, prefs }),
  hardwareVolume: (deviceId) => invoke<HardwareVolumeInfo | null>('hardware_volume', { deviceId }),
  setHardwareVolume: (deviceId, db) => invoke('set_hardware_volume', { deviceId, db }),
  setHardwareMute: (deviceId, muted) => invoke('set_hardware_mute', { deviceId, muted }),
  onSettingsChanged: (cb) => listen<AppSettings>('settings://changed', (e) => cb(e.payload)),

  onPlayerState: (cb) => listen<PlayerSnapshot>('player://state', (e) => cb(e.payload)),
  onPosition: (cb) => listen<{ positionMs: number }>('player://position', (e) => cb(e.payload.positionMs)),
  onPlayerError: (cb) => listen<PlayerError>('player://error', (e) => cb(e.payload)),
  onDevicesChanged: (cb) => listen<DeviceChange>('output://devices', (e) => cb(e.payload)),

  libraryOverview: () => invoke<LibraryOverview>('library_overview'),
  libraryFolders: () => invoke<LibraryFolder[]>('library_folders'),
  libraryAddFolder: (path) => invoke('library_add_folder', { path }),
  libraryRemoveFolder: (id) => invoke('library_remove_folder', { id }),
  libraryRescan: () => invoke('library_rescan'),
  libraryTracks: (sort, offset, limit) => invoke<TrackRow[]>('library_tracks', { sort, offset, limit }),
  libraryAlbums: (sort, offset, limit) => invoke<AlbumRow[]>('library_albums', { sort, offset, limit }),
  libraryAlbum: (id) => invoke<AlbumDetail | null>('library_album', { id }),
  libraryArtists: (offset, limit) => invoke<ArtistRow[]>('library_artists', { offset, limit }),
  libraryArtist: (id) => invoke<ArtistDetail | null>('library_artist', { id }),
  libraryGenres: () => invoke<GenreRow[]>('library_genres'),
  libraryGenre: (id) => invoke<GenreDetail | null>('library_genre', { id }),
  libraryFolder: (path) => invoke<FolderListing>('library_folder', { path }),
  librarySearch: (query, limit) => invoke<SearchResults>('library_search', { query, limit }),
  libraryDelimiters: () => invoke<string[]>('library_delimiters'),
  librarySetDelimiters: (delimiters) => invoke('library_set_delimiters', { delimiters }),
  playTracks: (ids, start, shuffle) => invoke('play_tracks', { ids, start, shuffle }),
  playLibrary: (sort, start, shuffle) => invoke('play_library', { sort, start, shuffle }),
  confirm: (message, title) => ask(message, { title, kind: 'warning' }),
  async pickFolder() {
    const defaultPath = await audioDir().catch(() => undefined);
    const picked = await open({ directory: true, multiple: false, title: 'Add a music folder', defaultPath });
    return typeof picked === 'string' ? picked : null;
  },
  artUrl: (hash, size) => convertFileSrc(`lib/${hash}-${size}`, 'cover'),
  onLibraryScan: (cb) => listen<ScanProgress>('library://scan', (e) => cb(e.payload)),
  onLibraryChanged: (cb) => listen('library://changed', () => cb()),

  setPreferences: (preferences) => invoke<AppSettings>('set_preferences', { preferences }),

  libraryPlaylists: () => invoke<PlaylistRow[]>('library_playlists'),
  libraryPlaylist: (id) => invoke<PlaylistDetail | null>('library_playlist', { id }),
  playlistCreate: (name, trackIds) => invoke<number>('playlist_create', { name, trackIds }),
  playlistRename: (id, name) => invoke('playlist_rename', { id, name }),
  playlistDelete: (id) => invoke('playlist_delete', { id }),
  playlistAddTracks: (id, trackIds) => invoke('playlist_add_tracks', { id, trackIds }),
  playlistRemove: (id, positions) => invoke('playlist_remove', { id, positions }),
  playlistMove: (id, from, to) => invoke('playlist_move', { id, from, to }),
  playlistImport: (path) => invoke<number>('playlist_import', { path }),
  playlistExport: (id, path) => invoke('playlist_export', { id, path }),
  playPlaylist: (id, start, shuffle) => invoke('play_playlist', { id, start, shuffle }),
  async pickPlaylistFile() {
    const picked = await open({
      multiple: false,
      directory: false,
      title: 'Import a playlist',
      filters: [{ name: 'Playlists', extensions: ['m3u8', 'm3u'] }],
    });
    return typeof picked === 'string' ? picked : null;
  },
  async pickPlaylistSaveFile(name) {
    const file = `${name.replace(/[\\/:*?"<>|]/g, '_')}.m3u8`;
    const defaultPath = await audioDir()
      .then((dir) => join(dir, file))
      .catch(() => file);
    const picked = await save({
      title: 'Export the playlist',
      defaultPath,
      filters: [{ name: 'M3U8 playlist', extensions: ['m3u8'] }],
    });
    return picked ?? null;
  },

  librarySetFavorite: (path, on) => invoke('library_set_favorite', { path, on }),
  libraryIsFavorite: (path) => invoke<boolean>('library_is_favorite', { path }),
  libraryFavorites: () => invoke<TrackRow[]>('library_favorites'),
  libraryStats: (days) => invoke<ListeningStats>('library_stats', { days }),
  libraryHome: () => invoke<HomeFeed>('library_home'),
  playDailyMix: (start) => invoke('play_daily_mix', { start }),

  lyricsFind: (track) => invoke<Lyrics | null>('lyrics_find', { track }),
  lyricsSave: (track, text) => invoke<Lyrics>('lyrics_save', { track, text }),

  async checkForUpdate(): Promise<UpdateInfo | null> {
    pendingUpdate = await check();
    return pendingUpdate ? { version: pendingUpdate.version, notes: pendingUpdate.body ?? null } : null;
  },
  async installUpdate(onProgress) {
    const update = pendingUpdate ?? (await check());
    if (!update) return;
    let total = 0;
    let done = 0;
    await update.downloadAndInstall((e) => {
      if (e.event === 'Started') total = e.data.contentLength ?? 0;
      if (e.event === 'Progress') {
        done += e.data.chunkLength;
        onProgress(total ? done / total : null);
      }
    });
    // The Windows installer usually closes the app itself; otherwise start the new version.
    await relaunch();
  },
  aiStatus: () => invoke<AiStatus>('ai_status'),
  aiSetKey: (key) => invoke('ai_set_key', { key }),
  aiRemoveKey: () => invoke('ai_remove_key'),
  aiPlaylist: (prompt, songs) => invoke<AiPlaylistResult>('ai_playlist', { prompt, songs }),
  openMiniPlayer: () => invoke('open_mini_player'),
  showMainWindow: () => invoke('show_main_window'),
  tagsRead: (paths) => invoke<TagFields[]>('tags_read', { paths }),
  tagsWrite: (paths, change) => invoke<TagWriteReport>('tags_write', { paths, change }),
};
