/** The context menus of tracks, albums and track lists. */
import { api, type AlbumRow, type TrackRow } from './api';
import { dialogs } from './dialogs.svelte';
import { playAll, queueRows, setFavorite, shuffleRows } from './library.svelte';
import type { MenuEntry } from './menu.svelte';
import { router } from './router.svelte';

/** The folder a file is in (either separator). */
export function parentDir(path: string): string {
  const i = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
  return i > 0 ? path.slice(0, i) : path;
}

const libraryIds = (rows: readonly TrackRow[]) => rows.filter((r) => r.id !== 0).map((r) => r.id);

export function trackMenu(track: TrackRow): MenuEntry[] {
  const inLibrary = track.id !== 0;
  const entries: MenuEntry[] = [
    { icon: 'queue_play_next', label: 'Play next', action: () => queueRows([track], true) },
    { icon: 'add_to_queue', label: 'Add to queue', action: () => queueRows([track], false) },
  ];
  if (inLibrary) {
    entries.push({ icon: 'playlist_add', label: 'Add to playlist…', action: () => dialogs.addToPlaylist([track.id], track.title) });
  }
  entries.push(
    track.favorite
      ? { icon: 'heart_minus', label: 'Remove from favourites', action: () => setFavorite(track, false) }
      : { icon: 'favorite', label: 'Add to favourites', action: () => setFavorite(track, true) },
  );
  if (inLibrary) {
    entries.push(
      'divider',
      { icon: 'album', label: 'Go to album', action: () => router.go({ screen: 'album', id: track.albumId }) },
      { icon: 'folder_open', label: 'Show in folder', action: () => router.go({ screen: 'folder', path: parentDir(track.path) }) },
    );
  }
  return entries;
}

/** Queue and playlist actions for a whole list (an album, artist or genre page). */
export function rowsMenu(rows: () => readonly TrackRow[], label: string): MenuEntry[] {
  return [
    { icon: 'queue_play_next', label: 'Play next', action: () => queueRows(rows(), true, label) },
    { icon: 'add_to_queue', label: 'Add to queue', action: () => queueRows(rows(), false, label) },
    { icon: 'playlist_add', label: 'Add to playlist…', action: () => dialogs.addToPlaylist(libraryIds(rows()), label) },
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
    { icon: 'add_to_queue', label: 'Add to queue', action: withTracks((t) => queueRows(t, false, title)) },
    { icon: 'playlist_add', label: 'Add to playlist…', action: withTracks((t) => dialogs.addToPlaylist(libraryIds(t), title)) },
    'divider',
    { icon: 'album', label: 'Go to album', action: () => router.go({ screen: 'album', id: album.id }) },
  ];
}
