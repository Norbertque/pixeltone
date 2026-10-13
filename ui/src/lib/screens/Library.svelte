<script lang="ts">
  import { untrack } from 'svelte';

  import { api, type AlbumSort, type GenreRow, type PlaylistRow, type TrackSort } from '../api';
  import AlbumCard from '../components/AlbumCard.svelte';
  import ArtistItem from '../components/ArtistItem.svelte';
  import Button from '../components/Button.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import GenreCard from '../components/GenreCard.svelte';
  import Icon from '../components/Icon.svelte';
  import PlaylistCard from '../components/PlaylistCard.svelte';
  import ScreenHeader from '../components/ScreenHeader.svelte';
  import TrackItem from '../components/TrackItem.svelte';
  import VirtualGrid from '../components/VirtualGrid.svelte';
  import VirtualList from '../components/VirtualList.svelte';
  import { dialogs } from '../dialogs.svelte';
  import { addMusicFolder, library, PagedRows } from '../library.svelte';
  import { formatDuration } from '../player.svelte';
  import { importPlaylist, newPlaylist } from '../playlists';
  import { router, type LibraryTab } from '../router.svelte';
  import FolderView from './FolderView.svelte';

  const TABS = [
    { id: 'songs', label: 'Songs', icon: 'music_note' },
    { id: 'albums', label: 'Albums', icon: 'album' },
    { id: 'artists', label: 'Artists', icon: 'artist' },
    { id: 'genres', label: 'Genres', icon: 'category' },
    { id: 'folders', label: 'Folders', icon: 'folder' },
    { id: 'playlists', label: 'Playlists', icon: 'queue_music' },
  ] as const satisfies readonly { id: LibraryTab; label: string; icon: string }[];

  const TRACK_SORTS: { value: TrackSort; label: string }[] = [
    { value: 'title', label: 'Title' },
    { value: 'artist', label: 'Artist' },
    { value: 'album', label: 'Album' },
    { value: 'year', label: 'Year' },
    { value: 'added', label: 'Recently added' },
    { value: 'duration', label: 'Longest' },
  ];
  const ALBUM_SORTS: { value: AlbumSort; label: string }[] = [
    { value: 'title', label: 'Title' },
    { value: 'artist', label: 'Artist' },
    { value: 'year', label: 'Year' },
    { value: 'added', label: 'Recently added' },
  ];

  let trackSort = $state<TrackSort>('title');
  let albumSort = $state<AlbumSort>('title');
  let genres = $state<GenreRow[]>([]);

  const o = $derived(library.overview);
  const tab = $derived(router.libraryTab);

  // A pager per sort order. When the library or user data changes, the pagers reload in
  // place: rows stay on screen until their new versions arrive (no flash on a heart toggle).
  const songs = $derived.by(() => {
    const sort = trackSort;
    const count = untrack(() => o?.trackCount ?? 0);
    return new PagedRows(count, (offset, limit) => api.libraryTracks(sort, offset, limit));
  });
  const albums = $derived.by(() => {
    const sort = albumSort;
    const count = untrack(() => o?.albumCount ?? 0);
    return new PagedRows(count, (offset, limit) => api.libraryAlbums(sort, offset, limit));
  });
  const artists = new PagedRows(0, (offset, limit) => api.libraryArtists(offset, limit));

  $effect(() => {
    void library.version;
    const counts = [o?.trackCount ?? 0, o?.albumCount ?? 0, o?.artistCount ?? 0] as const;
    untrack(() => {
      songs.refresh(counts[0]);
      albums.refresh(counts[1]);
      artists.refresh(counts[2]);
    });
  });

  let playlists = $state<PlaylistRow[]>([]);
  let favourites = $state<{ count: number; covers: string[] }>({ count: 0, covers: [] });
  $effect(() => {
    void library.version;
    if (tab !== 'playlists') return;
    void api.libraryPlaylists().then((p) => (playlists = p));
    void api.libraryFavorites().then((f) => {
      favourites = { count: f.length, covers: [...new Set(f.map((t) => t.cover).filter((c): c is string => !!c))].slice(0, 4) };
    });
  });

  $effect(() => {
    void library.version;
    if (tab === 'genres') void api.libraryGenres().then((g) => (genres = g));
  });

  function onkeydown(e: KeyboardEvent): void {
    const i = TABS.findIndex((t) => t.id === tab);
    const next = e.key === 'ArrowRight' ? i + 1 : e.key === 'ArrowLeft' ? i - 1 : null;
    if (next === null) return;
    e.preventDefault();
    const target = TABS[(next + TABS.length) % TABS.length]!;
    router.libraryTab = target.id;
    document.getElementById(`tab-${target.id}`)?.focus();
  }

  const subtitle = $derived(
    o && o.trackCount > 0
      ? `${o.trackCount.toLocaleString()} songs · ${o.albumCount.toLocaleString()} albums · ${formatDuration(o.durationMs)}`
      : undefined,
  );
</script>

<ScreenHeader title="Library" {subtitle}>
  {#snippet actions()}
    {#if o && o.trackCount > 0}
      <Button variant="tonal" icon="shuffle" onclick={() => api.playLibrary('title', null, true)}>Shuffle all</Button>
    {/if}
  {/snippet}
</ScreenHeader>

{#if library.scanning && library.scan}
  <p class="scanning body-medium" role="status">
    <Icon name="sync" size={18} />
    Scanning your music… {library.scan.done.toLocaleString()} of {library.scan.total.toLocaleString()} new or changed files
  </p>
{/if}

<div class="tabs" role="tablist" aria-label="Library sections" tabindex="-1" {onkeydown}>
  {#each TABS as t (t.id)}
    {@const selected = t.id === tab}
    <button
      id="tab-{t.id}"
      role="tab"
      aria-selected={selected}
      aria-controls="library-panel"
      tabindex={selected ? 0 : -1}
      class={['tab', { selected }]}
      onclick={() => (router.libraryTab = t.id)}
    >
      <Icon name={t.icon} size={20} fill={selected} />
      <span class="label-large">{t.label}</span>
    </button>
  {/each}
</div>

<div id="library-panel" role="tabpanel" aria-labelledby="tab-{tab}" class="panel">
  {#if !o || (o.trackCount === 0 && tab !== 'folders' && tab !== 'playlists')}
    {#if o && o.folders.length === 0}
      <EmptyState
        icon="library_music"
        title="Add your music"
        body="Choose the folders where your music lives. PixelPlayer reads them once and keeps up with changes."
      >
        {#snippet action()}
          <Button variant="filled" icon="create_new_folder" onclick={addMusicFolder}>Add music folder</Button>
        {/snippet}
      </EmptyState>
    {:else if o}
      <EmptyState
        icon="hourglass_top"
        title={library.scanning ? 'Reading your music…' : 'No music found'}
        body={library.scanning
          ? 'Your songs will appear here in a moment.'
          : 'None of the library folders contain music files PixelPlayer can play.'}
      />
    {/if}
  {:else if tab === 'songs'}
    <div class="toolbar">
      <span class="label-large count">{o.trackCount.toLocaleString()} songs</span>
      <label class="sort label-large">
        Sort
        <select bind:value={trackSort} class="body-medium">
          {#each TRACK_SORTS as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
        </select>
      </label>
    </div>
    <VirtualList
      count={songs.count}
      rowHeight={60}
      label="Songs"
      onrange={(s, e) => songs.ensure(s, e)}
    >
      {#snippet row(i)}
        <TrackItem track={songs.rows[i]} onplay={() => api.playLibrary(trackSort, i, null)} />
      {/snippet}
    </VirtualList>
  {:else if tab === 'albums'}
    <div class="toolbar">
      <span class="label-large count">{o.albumCount.toLocaleString()} albums</span>
      <label class="sort label-large">
        Sort
        <select bind:value={albumSort} class="body-medium">
          {#each ALBUM_SORTS as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
        </select>
      </label>
    </div>
    <VirtualGrid count={albums.count} label="Albums" onrange={(s, e) => albums.ensure(s, e)}>
      {#snippet item(i, width)}
        <AlbumCard album={albums.rows[i]} size={width} />
      {/snippet}
    </VirtualGrid>
  {:else if tab === 'artists'}
    <div class="toolbar">
      <span class="label-large count">{o.artistCount.toLocaleString()} artists</span>
    </div>
    <VirtualList count={artists.count} rowHeight={64} label="Artists" onrange={(s, e) => artists.ensure(s, e)}>
      {#snippet row(i)}
        <ArtistItem artist={artists.rows[i]} />
      {/snippet}
    </VirtualList>
  {:else if tab === 'genres'}
    <div class="genres">
      {#each genres as g (g.id)}
        <GenreCard genre={g} />
      {/each}
    </div>
  {:else if tab === 'folders'}
    <FolderView path={null} />
  {:else}
    <div class="toolbar">
      <span class="label-large count">{playlists.length.toLocaleString()} playlist{playlists.length === 1 ? '' : 's'}</span>
      <div class="toolbar-actions">
        <Button variant="tonal" icon="add" onclick={newPlaylist}>New playlist</Button>
        <Button variant="tonal" icon="auto_awesome" onclick={() => (dialogs.ai = true)}>Create with AI</Button>
        <Button variant="text" icon="upload_file" onclick={importPlaylist}>Import M3U</Button>
      </div>
    </div>
    <div class="playlists">
      <PlaylistCard {favourites} />
      {#each playlists as p (p.id)}
        <PlaylistCard playlist={p} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .scanning {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: -8px 0 12px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .tabs {
    position: sticky;
    top: 0;
    z-index: 2;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0 -8px;
    padding: 8px;
    background: var(--md-sys-color-surface);
  }
  .tab {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 16px 0 12px;
    border: none;
    border-radius: var(--shape-full);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    transition:
      background-color var(--motion-effects-default-duration) var(--motion-effects-default),
      border-radius var(--motion-spatial-fast-duration) var(--motion-spatial-fast);
  }
  .tab:hover {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, var(--md-sys-color-surface-container-high));
  }
  .tab.selected {
    border-radius: var(--shape-md);
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .panel {
    padding-bottom: 32px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 8px;
  }
  .count {
    color: var(--md-sys-color-on-surface-variant);
  }
  .sort {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .sort select {
    height: 36px;
    padding: 0 12px;
    border: none;
    border-radius: var(--shape-full);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
  }
  .toolbar-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .playlists {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 20px;
    padding-bottom: 32px;
  }
  .genres {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
    padding-top: 12px;
  }
</style>
