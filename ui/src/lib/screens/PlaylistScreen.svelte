<script lang="ts">
  import { api, type PlaylistDetail } from '../api';
  import Button from '../components/Button.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import Icon from '../components/Icon.svelte';
  import IconButton from '../components/IconButton.svelte';
  import Mosaic from '../components/Mosaic.svelte';
  import ReorderList from '../components/ReorderList.svelte';
  import TrackItem from '../components/TrackItem.svelte';
  import { library } from '../library.svelte';
  import { menu } from '../menu.svelte';
  import { formatDuration } from '../player.svelte';
  import { playlistMenu } from '../playlists';
  import type { HandleProps } from '../queue/drag';
  import { router } from '../router.svelte';
  import { toasts } from '../toasts.svelte';

  interface Props {
    id: number;
  }
  let { id }: Props = $props();

  let detail = $state<PlaylistDetail | null>(null);
  let missing = $state(false);

  $effect(() => {
    void library.version;
    const wanted = id;
    void api.libraryPlaylist(wanted).then((d) => {
      if (wanted !== id) return;
      detail = d;
      missing = d === null;
    });
  });

  const ROW = 60;

  function move(from: number, to: number): void {
    if (!detail) return;
    const tracks = detail.tracks.slice();
    const [t] = tracks.splice(from, 1);
    if (!t) return;
    tracks.splice(to, 0, t);
    detail = { ...detail, tracks };
    api.playlistMove(id, from, to).catch((e) => toasts.show(String(e), 'error'));
  }

  function remove(i: number): void {
    api.playlistRemove(id, [i]).catch((e) => toasts.show(String(e), 'error'));
  }
</script>

{#snippet row(i: number, handle: HandleProps)}
  {@const t = detail!.tracks[i]}
  {#if t}
    <div class="prow">
      <button type="button" class="handle" aria-label="Move {t.title} (up and down arrow keys)" title="Drag to reorder" {...handle}>
        <Icon name="drag_indicator" size={20} />
      </button>
      <TrackItem track={t} onplay={() => api.playPlaylist(id, i, null)} />
      <IconButton icon="remove_circle_outline" label="Remove {t.title} from the playlist" size="xs" onclick={() => remove(i)} />
    </div>
  {/if}
{/snippet}

{#snippet ghost(i: number)}
  {@const t = detail!.tracks[i]}
  {#if t}
    <div class="prow lifted">
      <span class="handle"><Icon name="drag_indicator" size={20} /></span>
      <span class="ghost-title title-small">{t.title}</span>
    </div>
  {/if}
{/snippet}

{#if detail}
  {@const p = detail.playlist}
  <header class="hero">
    <div class="cover"><Mosaic covers={p.covers} radius="var(--shape-2xl)" /></div>
    <div class="info">
      <span class="overline label-large">Playlist</span>
      <h1 class="display-small rounded emphasized" tabindex="-1" data-screen-heading>{p.name}</h1>
      <p class="meta body-medium">
        {p.trackCount.toLocaleString()} song{p.trackCount === 1 ? '' : 's'}{p.durationMs > 0 ? ` · ${formatDuration(p.durationMs)}` : ''}
      </p>
      <div class="actions">
        <Button variant="filled" size="md" icon="play_arrow" disabled={!p.trackCount} onclick={() => api.playPlaylist(id, 0, false)}>Play</Button>
        <Button variant="tonal" size="md" icon="shuffle" disabled={!p.trackCount} onclick={() => api.playPlaylist(id, null, true)}>Shuffle</Button>
        <IconButton
          icon="more_vert"
          label="More options"
          variant="tonal"
          size="md"
          onclick={(e) => menu.show(e.currentTarget, playlistMenu(p, () => void router.back()), p.name)}
        />
      </div>
    </div>
  </header>

  {#if detail.tracks.length}
    <section class="tracks" aria-label="Songs">
      <ReorderList
        count={detail.tracks.length}
        rowHeight={ROW}
        label="Songs in {p.name}"
        {row}
        {ghost}
        keyOf={(i) => detail?.tracks[i]?.path}
        describe={(i) => `${detail?.tracks[i]?.title ?? 'Song'} moved to position ${i + 1}`}
        onmove={move}
      />
    </section>
  {:else}
    <EmptyState icon="queue_music" title="This playlist is empty" body="Use “Add to playlist…” on any song or album." />
  {/if}
{:else if missing}
  <EmptyState icon="queue_music" title="Playlist not found" body="It may have been deleted." />
{/if}

<style>
  .hero {
    display: grid;
    grid-template-columns: minmax(140px, 220px) 1fr;
    align-items: end;
    gap: 32px;
    margin: 16px 0 24px;
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
  .meta {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
  }
  .tracks {
    padding-bottom: 32px;
  }
  .prow {
    display: grid;
    grid-template-columns: 32px 1fr 36px;
    align-items: center;
    height: 100%;
    border-radius: var(--shape-md);
  }
  .prow.lifted {
    grid-template-columns: 32px 1fr;
    padding-left: 0;
  }
  .ghost-title {
    padding-left: 16px;
  }
  .handle {
    display: grid;
    place-items: center;
    width: 32px;
    height: 48px;
    padding: 0;
    border: none;
    border-radius: var(--shape-sm);
    background: none;
    color: var(--md-sys-color-on-surface-variant);
    cursor: grab;
    touch-action: none;
    -webkit-user-select: none;
    user-select: none;
  }
  .handle:focus-visible {
    outline: 2px solid var(--md-sys-color-secondary);
  }
  @media (width < 700px) {
    .hero {
      grid-template-columns: 1fr;
    }
    .cover {
      width: min(220px, 60%);
    }
  }
</style>
