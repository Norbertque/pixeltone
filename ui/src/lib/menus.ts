/** The context menus of tracks, albums and track lists. */
import { api, type AlbumRow, type TrackRow } from './api';
import { playAll, queueRows, shuffleRows } from './library.svelte';
import type { MenuEntry } from './menu.svelte';
import { router } from './router.svelte';

/** The folder a file is in (either separator). */
export function parentDir(path: string): string {
  const i = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
  return i > 0 ? path.slice(0, i) : path;
}

export function trackMenu(track: TrackRow): MenuEntry[] {
  return [
    { icon: 'queue_play_next', label: 'Play next', action: () => queueRows([track], true) },
    { icon: 'playlist_add', label: 'Add to queue', action: () => queueRows([track], false) },
    'divider',
    { icon: 'album', label: 'Go to album', action: () => router.go({ screen: 'album', id: track.albumId }) },
    { icon: 'folder_open', label: 'Show in folder', action: () => router.go({ screen: 'folder', path: parentDir(track.path) }) },
  ];
}

/** "Play next" and "Add to queue" for a whole list (an album, artist or genre page). */
export function rowsMenu(rows: () => readonly TrackRow[], label: string): MenuEntry[] {
  return [
    { icon: 'queue_play_next', label: 'Play next', action: () => queueRows(rows(), true, label) },
    { icon: 'playlist_add', label: 'Add to queue', action: () => queueRows(rows(), false, label) },
  ];
}

export function albumMenu(album: AlbumRow): MenuEntry[] {
  const title = album.title || 'Unknown album';
  const withTracks = (f: (rows: TrackRow[]) => unknown) => async () => {
    const detail = await api.libraryAlbum(album.id);
    if (detail) await f(detail.tracks);
  };
  return [
    { icon: 'play_arrow', label: 'Play', action: withTracks(playAll) },
    { icon: 'shuffle', label: 'Shuffle', action: withTracks(shuffleRows) },
    'divider',
    { icon: 'queue_play_next', label: 'Play next', action: withTracks((t) => queueRows(t, true, title)) },
    { icon: 'playlist_add', label: 'Add to queue', action: withTracks((t) => queueRows(t, false, title)) },
    'divider',
    { icon: 'album', label: 'Go to album', action: () => router.go({ screen: 'album', id: album.id }) },
  ];
}
