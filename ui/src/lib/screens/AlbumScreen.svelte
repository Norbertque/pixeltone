<script lang="ts">
  import { api, type AlbumDetail } from '../api';
  import Artwork from '../components/Artwork.svelte';
  import Button from '../components/Button.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import IconButton from '../components/IconButton.svelte';
  import TrackItem from '../components/TrackItem.svelte';
  import { library, playAll, playRows, shuffleRows } from '../library.svelte';
  import { menu } from '../menu.svelte';
  import { rowsMenu } from '../menus';
  import { coverOf, formatDuration, formatShort, seedFromImage } from '../player.svelte';
  import { theme } from '../theme/theme.svelte';
  import { tintStyle } from '../theme/tint';

  interface Props {
    id: number;
  }
  let { id }: Props = $props();

  let detail = $state<AlbumDetail | null>(null);
  let missing = $state(false);
  let seed = $state<number | null>(null);

  $effect(() => {
    void library.version;
    const wanted = id;
    void api.libraryAlbum(wanted).then(async (d) => {
      if (wanted !== id) return;
      detail = d;
      missing = d === null;
      seed = d?.album.cover ? await seedFromImage(api.artUrl(d.album.cover, 300)) : null;
    });
  });

  // The page takes on the album's own colours, whatever the app theme is following.
  const tint = $derived(tintStyle(seed, theme.dark, theme.style, theme.contrast));

  const discs = $derived.by(() => {
    if (!detail) return [];
    const groups = new Map<number, { index: number; track: AlbumDetail['tracks'][number] }[]>();
    detail.tracks.forEach((track, index) => {
      const disc = track.discNo ?? 1;
      groups.set(disc, [...(groups.get(disc) ?? []), { index, track }]);
    });
    return [...groups].sort(([a], [b]) => a - b);
  });

  const formats = $derived(detail ? [...new Set(detail.tracks.map(formatShort))].join(', ') : '');
</script>

{#if detail}
  {@const album = detail.album}
  <div class="album" style={tint}>
    <header class="hero">
      <div class="cover">
        <Artwork src={coverOf(album.cover, 1000)} alt="Cover of {album.title}" size="100%" radius="var(--shape-2xl)" />
      </div>
      <div class="info">
        <span class="overline label-large">Album</span>
        <h1 class="display-small rounded emphasized" tabindex="-1" data-screen-heading>
          {album.title || 'Unknown album'}
        </h1>
        <p class="artist title-large">{album.artist || 'Unknown artist'}</p>
        <p class="meta body-medium">
          {[album.year, `${album.trackCount} song${album.trackCount === 1 ? '' : 's'}`, formatDuration(album.durationMs), formats]
            .filter(Boolean)
            .join(' · ')}
        </p>
        <div class="actions">
          <Button variant="filled" size="md" icon="play_arrow" onclick={() => playAll(detail!.tracks)}>Play</Button>
          <Button variant="tonal" size="md" icon="shuffle" onclick={() => shuffleRows(detail!.tracks)}>Shuffle</Button>
          <IconButton
            icon="more_vert"
            label="More options"
            variant="tonal"
            size="md"
            onclick={(e) => menu.show(e.currentTarget, rowsMenu(() => detail!.tracks, album.title || 'Unknown album'), 'More options')}
          />
        </div>
      </div>
    </header>

    <section class="tracks" aria-label="Songs">
      {#each discs as [disc, rows] (disc)}
        {#if discs.length > 1}
          <h2 class="disc title-small">Disc {disc}</h2>
        {/if}
        {#each rows as { index, track } (track.id)}
          <div class="row">
            <TrackItem {track} numbered showAlbum={false} onplay={() => playRows(detail!.tracks, index)} />
          </div>
        {/each}
      {/each}
    </section>
  </div>
{:else if missing}
  <EmptyState icon="album" title="Album not found" body="It may have been removed from the library." />
{/if}

<style>
  .album {
    margin: 16px 0 32px;
    padding: 24px;
    border-radius: var(--shape-2xl);
    background:
      radial-gradient(120% 80% at 0% 0%, color-mix(in srgb, var(--md-sys-color-primary-container) 75%, transparent), transparent 70%),
      var(--md-sys-color-surface-container-low);
    color: var(--md-sys-color-on-surface);
  }
  .hero {
    container: hero / inline-size;
    display: grid;
    grid-template-columns: minmax(160px, 260px) 1fr;
    align-items: end;
    gap: 32px;
    margin-bottom: 24px;
  }
  .cover {
    filter: drop-shadow(0 10px 28px rgb(0 0 0 / 0.25));
  }
  .info {
    display: grid;
    gap: 6px;
    min-width: 0;
  }
  .overline {
    color: var(--md-sys-color-primary);
    text-transform: uppercase;
    letter-spacing: 1px;
  }
  h1 {
    margin: 0;
    overflow-wrap: anywhere;
  }
  h1:focus {
    outline: none;
  }
  .artist,
  .meta {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: 16px;
  }
  .disc {
    margin: 16px 8px 8px;
    color: var(--md-sys-color-primary);
  }
  .row {
    height: 60px;
  }
  @container hero (width < 560px) {
    .hero {
      grid-template-columns: 1fr;
    }
    .cover {
      width: min(260px, 70%);
    }
  }
</style>
