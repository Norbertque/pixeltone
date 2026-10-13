<script lang="ts">
  import { api, type TrackRow } from '../api';
  import Button from '../components/Button.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import Icon from '../components/Icon.svelte';
  import TrackItem from '../components/TrackItem.svelte';
  import VirtualList from '../components/VirtualList.svelte';
  import { library, playAll, playRows, shuffleRows } from '../library.svelte';
  import { formatDuration } from '../player.svelte';

  let tracks = $state<TrackRow[] | null>(null);

  $effect(() => {
    void library.version;
    void api.libraryFavorites().then((t) => (tracks = t));
  });

  const total = $derived(tracks?.reduce((s, t) => s + t.durationMs, 0) ?? 0);
</script>

<header class="hero">
  <span class="art" aria-hidden="true"><Icon name="favorite" fill size={64} /></span>
  <div class="info">
    <span class="overline label-large">Playlist</span>
    <h1 class="display-small rounded emphasized" tabindex="-1" data-screen-heading>Favourites</h1>
    {#if tracks}
      <p class="meta body-medium">
        {tracks.length.toLocaleString()} song{tracks.length === 1 ? '' : 's'}{total > 0 ? ` · ${formatDuration(total)}` : ''}
      </p>
    {/if}
    <div class="actions">
      <Button variant="filled" size="md" icon="play_arrow" disabled={!tracks?.length} onclick={() => tracks && playAll(tracks)}>Play</Button>
      <Button variant="tonal" size="md" icon="shuffle" disabled={!tracks?.length} onclick={() => tracks && shuffleRows(tracks)}>Shuffle</Button>
    </div>
  </div>
</header>

{#if tracks && tracks.length === 0}
  <EmptyState icon="favorite" title="No favourites yet" body="Tap the heart in Now Playing, or right-click any song." />
{:else if tracks}
  <div class="tracks">
    <VirtualList count={tracks.length} rowHeight={60} label="Favourite songs">
      {#snippet row(i)}
        <TrackItem track={tracks![i]} onplay={() => playRows(tracks!, i)} />
      {/snippet}
    </VirtualList>
  </div>
{/if}

<style>
  .hero {
    display: grid;
    grid-template-columns: minmax(140px, 220px) 1fr;
    align-items: end;
    gap: 32px;
    margin: 16px 0 24px;
  }
  .art {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    border-radius: var(--shape-2xl);
    background: linear-gradient(135deg, var(--md-sys-color-primary-container), var(--md-sys-color-tertiary-container));
    color: var(--md-sys-color-on-primary-container);
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
  .tracks {
    padding-bottom: 32px;
  }
  @media (width < 700px) {
    .hero {
      grid-template-columns: 1fr;
    }
    .art {
      width: min(220px, 60%);
    }
  }
</style>
