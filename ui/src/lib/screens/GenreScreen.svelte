<script lang="ts">
  import { api, type GenreDetail } from '../api';
  import AlbumCard from '../components/AlbumCard.svelte';
  import Button from '../components/Button.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import TrackItem from '../components/TrackItem.svelte';
  import VirtualList from '../components/VirtualList.svelte';
  import { library, playRows, shuffleRows } from '../library.svelte';

  interface Props {
    id: number;
  }
  let { id }: Props = $props();

  let detail = $state<GenreDetail | null>(null);
  let missing = $state(false);

  $effect(() => {
    void library.version;
    const wanted = id;
    void api.libraryGenre(wanted).then((d) => {
      if (wanted !== id) return;
      detail = d;
      missing = d === null;
    });
  });
</script>

{#if detail}
  <header class="hero">
    <span class="overline label-large">Genre</span>
    <h1 class="display-medium rounded emphasized" tabindex="-1" data-screen-heading>{detail.genre.name}</h1>
    <p class="meta body-medium">
      {detail.albums.length} album{detail.albums.length === 1 ? '' : 's'} · {detail.genre.trackCount} songs
    </p>
    <div class="actions">
      <Button variant="filled" size="md" icon="play_arrow" onclick={() => playRows(detail!.tracks, 0)}>Play</Button>
      <Button variant="tonal" size="md" icon="shuffle" onclick={() => shuffleRows(detail!.tracks)}>Shuffle</Button>
    </div>
  </header>

  <h2 class="section title-large rounded emphasized">Albums</h2>
  <div class="albums">
    {#each detail.albums as album (album.id)}
      <AlbumCard {album} />
    {/each}
  </div>

  <h2 class="section title-large rounded emphasized">Songs</h2>
  <div class="songs">
    <VirtualList count={detail.tracks.length} rowHeight={60} label="Songs">
      {#snippet row(i)}
        <TrackItem track={detail!.tracks[i]} onplay={() => playRows(detail!.tracks, i)} />
      {/snippet}
    </VirtualList>
  </div>
{:else if missing}
  <EmptyState icon="category" title="Genre not found" />
{/if}

<style>
  .hero {
    display: grid;
    gap: 6px;
    margin: 16px 0;
    padding: 32px;
    border-radius: var(--shape-2xl);
    background: var(--md-sys-color-tertiary-container);
    color: var(--md-sys-color-on-tertiary-container);
  }
  .overline {
    text-transform: uppercase;
    letter-spacing: 1px;
    opacity: 0.8;
  }
  h1 {
    margin: 0;
  }
  h1:focus {
    outline: none;
  }
  .meta {
    margin: 0;
    opacity: 0.8;
  }
  .actions {
    display: flex;
    gap: 12px;
    margin-top: 12px;
  }
  .section {
    margin: 24px 0 12px;
  }
  .albums {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 20px;
  }
  .songs {
    padding-bottom: 32px;
  }
</style>
