<script lang="ts">
  import { api, type SearchResults } from '../api';
  import AlbumCard from '../components/AlbumCard.svelte';
  import ArtistItem from '../components/ArtistItem.svelte';
  import Button from '../components/Button.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import Icon from '../components/Icon.svelte';
  import ScreenHeader from '../components/ScreenHeader.svelte';
  import TrackItem from '../components/TrackItem.svelte';
  import { addMusicFolder, library, playRows } from '../library.svelte';

  let query = $state('');
  let results = $state<SearchResults | null>(null);
  let searchedFor = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const q = query.trim();
    void library.version;
    clearTimeout(timer);
    if (!q) {
      results = null;
      searchedFor = '';
      return;
    }
    timer = setTimeout(async () => {
      const r = await api.librarySearch(q, 30);
      if (query.trim() === q) {
        results = r;
        searchedFor = q;
      }
    }, 150);
  });

  const nothing = $derived(
    results !== null && results.tracks.length + results.albums.length + results.artists.length === 0,
  );
</script>

<ScreenHeader title="Search" />

<label class="search">
  <Icon name="search" />
  <!-- svelte-ignore a11y_autofocus -->
  <input
    type="search"
    placeholder="Songs, albums, artists"
    bind:value={query}
    class="body-large"
    aria-label="Search your library"
    autofocus
  />
  {#if query}
    <button class="clear" onclick={() => (query = '')} aria-label="Clear search"><Icon name="close" /></button>
  {/if}
</label>

{#if library.empty && !query}
  <EmptyState
    icon="manage_search"
    title="Nothing to search yet"
    body="Add a music folder and every song, album and artist becomes searchable."
  >
    {#snippet action()}
      <Button variant="tonal" icon="create_new_folder" onclick={addMusicFolder}>Add music folder</Button>
    {/snippet}
  </EmptyState>
{:else if nothing}
  <EmptyState icon="search_off" title="No results" body="Nothing matches “{searchedFor}”." />
{:else if results}
  <div class="results">
    {#if results.artists.length > 0}
      <section>
        <h2 class="title-large rounded emphasized">Artists</h2>
        <div class="artists">
          {#each results.artists.slice(0, 6) as a (a.id)}
            <div class="artist-row"><ArtistItem artist={a} /></div>
          {/each}
        </div>
      </section>
    {/if}
    {#if results.albums.length > 0}
      <section>
        <h2 class="title-large rounded emphasized">Albums</h2>
        <div class="albums">
          {#each results.albums.slice(0, 12) as album (album.id)}
            <AlbumCard {album} />
          {/each}
        </div>
      </section>
    {/if}
    {#if results.tracks.length > 0}
      <section>
        <h2 class="title-large rounded emphasized">Songs</h2>
        {#each results.tracks as t, i (t.id)}
          <div class="row"><TrackItem track={t} onplay={() => playRows(results!.tracks, i)} /></div>
        {/each}
      </section>
    {/if}
  </div>
{/if}

<style>
  .search {
    position: sticky;
    top: 8px;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: 720px;
    height: 56px;
    padding: 0 12px 0 20px;
    border-radius: var(--shape-full);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface-variant);
    user-select: text;
  }
  .search:focus-within {
    outline: 2px solid var(--md-sys-color-primary);
  }
  input {
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    color: var(--md-sys-color-on-surface);
    outline: none;
  }
  input::-webkit-search-cancel-button {
    display: none;
  }
  input::placeholder {
    color: var(--md-sys-color-on-surface-variant);
  }
  .clear {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border: none;
    border-radius: 50%;
    background: none;
    color: inherit;
    cursor: pointer;
  }
  .clear:hover {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent);
  }
  .results {
    display: grid;
    gap: 24px;
    padding: 24px 0 32px;
  }
  h2 {
    margin: 0 0 12px;
  }
  .artists {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 4px;
  }
  .artist-row {
    height: 64px;
  }
  .albums {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 20px;
  }
  .row {
    height: 60px;
  }
</style>
