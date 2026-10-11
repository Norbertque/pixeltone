<script lang="ts">
  import { api, type ArtistDetail } from '../api';
  import AlbumCard from '../components/AlbumCard.svelte';
  import Artwork from '../components/Artwork.svelte';
  import Button from '../components/Button.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import TrackItem from '../components/TrackItem.svelte';
  import VirtualList from '../components/VirtualList.svelte';
  import { library, playRows, shuffleRows } from '../library.svelte';
  import { coverOf } from '../player.svelte';

  interface Props {
    id: number;
  }
  let { id }: Props = $props();

  let detail = $state<ArtistDetail | null>(null);
  let missing = $state(false);

  $effect(() => {
    void library.version;
    const wanted = id;
    void api.libraryArtist(wanted).then((d) => {
      if (wanted !== id) return;
      detail = d;
      missing = d === null;
    });
  });
</script>

{#if detail}
  {@const artist = detail.artist}
  <header class="hero">
    <Artwork src={coverOf(artist.cover, 1000)} alt="" size="168px" radius="var(--shape-full)" />
    <div class="info">
      <span class="overline label-large">Artist</span>
      <h1 class="display-small rounded emphasized" tabindex="-1" data-screen-heading>{artist.name}</h1>
      <p class="meta body-medium">
        {artist.albumCount} album{artist.albumCount === 1 ? '' : 's'} · {artist.trackCount} song{artist.trackCount === 1
          ? ''
          : 's'}
      </p>
      <div class="actions">
        <Button variant="filled" size="md" icon="play_arrow" onclick={() => playRows(detail!.tracks, 0)}>Play</Button>
        <Button variant="tonal" size="md" icon="shuffle" onclick={() => shuffleRows(detail!.tracks)}>Shuffle</Button>
      </div>
    </div>
  </header>

  {#if detail.albums.length > 0}
    <h2 class="section title-large rounded emphasized">Albums</h2>
    <div class="albums">
      {#each detail.albums as album (album.id)}
        <AlbumCard {album} />
      {/each}
    </div>
  {/if}

  <h2 class="section title-large rounded emphasized">Songs</h2>
  <div class="songs">
    <VirtualList count={detail.tracks.length} rowHeight={60} label="Songs">
      {#snippet row(i)}
        <TrackItem track={detail!.tracks[i]} onplay={() => playRows(detail!.tracks, i)} />
      {/snippet}
    </VirtualList>
  </div>
{:else if missing}
  <EmptyState icon="artist" title="Artist not found" body="They may have been removed from the library." />
{/if}

<style>
  .hero {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 32px;
    padding: 32px 0 16px;
  }
  .info {
    display: grid;
    gap: 6px;
  }
  .overline {
    color: var(--md-sys-color-primary);
    text-transform: uppercase;
    letter-spacing: 1px;
  }
  h1 {
    margin: 0;
  }
  h1:focus {
    outline: none;
  }
  .meta {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
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
