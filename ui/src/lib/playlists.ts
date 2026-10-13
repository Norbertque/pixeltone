/** Playlist actions shared by the Library tab, playlist cards and the playlist page. */
import { api, type PlaylistRow } from './api';
import { dialogs } from './dialogs.svelte';
import type { MenuEntry } from './menu.svelte';
import { router } from './router.svelte';
import { toasts } from './toasts.svelte';

async function attempt(work: () => Promise<unknown>): Promise<void> {
  try {
    await work();
  } catch (e) {
    toasts.show(String(e), 'error');
  }
}

export async function newPlaylist(): Promise<void> {
  const name = await dialogs.ask('New playlist', { label: 'Name', confirm: 'Create' });
  if (!name) return;
  await attempt(async () => {
    const id = await api.playlistCreate(name, []);
    await router.go({ screen: 'playlist', id });
  });
}

export async function importPlaylist(): Promise<void> {
  const path = await api.pickPlaylistFile();
  if (!path) return;
  await attempt(async () => {
    const id = await api.playlistImport(path);
    toasts.show('Imported the playlist');
    await router.go({ screen: 'playlist', id });
  });
}

export async function renamePlaylist(p: Pick<PlaylistRow, 'id' | 'name'>): Promise<void> {
  const name = await dialogs.ask('Rename playlist', { label: 'Name', confirm: 'Rename', value: p.name });
  if (!name || name === p.name) return;
  await attempt(() => api.playlistRename(p.id, name));
}

export async function deletePlaylist(p: Pick<PlaylistRow, 'id' | 'name'>, after?: () => void): Promise<void> {
  const ok = await api.confirm(`Delete “${p.name}”? The songs stay in your library.`, 'Delete playlist');
  if (!ok) return;
  await attempt(async () => {
    await api.playlistDelete(p.id);
    toasts.show(`Deleted “${p.name}”`);
    after?.();
  });
}

export async function exportPlaylist(p: Pick<PlaylistRow, 'id' | 'name'>): Promise<void> {
  const path = await api.pickPlaylistSaveFile(p.name);
  if (!path) return;
  await attempt(async () => {
    await api.playlistExport(p.id, path);
    toasts.show(`Exported to ${path}`);
  });
}

export function playlistMenu(p: Pick<PlaylistRow, 'id' | 'name'>, after?: () => void): MenuEntry[] {
  return [
    { icon: 'play_arrow', label: 'Play', action: () => api.playPlaylist(p.id, 0, false) },
    { icon: 'shuffle', label: 'Shuffle', action: () => api.playPlaylist(p.id, null, true) },
    'divider',
    { icon: 'edit', label: 'Rename…', action: () => renamePlaylist(p) },
    { icon: 'file_save', label: 'Export as M3U8…', action: () => exportPlaylist(p) },
    { icon: 'delete', label: 'Delete…', action: () => deletePlaylist(p, after) },
  ];
}
