<script lang="ts">
  import { api, type PlaylistRow } from '../api';
  import { contextMenu } from '../menu.svelte';
  import { playlistMenu } from '../playlists';
  import { router } from '../router.svelte';
  import Icon from './Icon.svelte';
  import Mosaic from './Mosaic.svelte';

  interface Props {
    /** A playlist, or the favourites when absent. */
    playlist?: PlaylistRow;
    favourites?: { count: number; covers: string[] };
  }
  let { playlist, favourites }: Props = $props();

  const name = $derived(playlist?.name ?? 'Favourites');
  const count = $derived(playlist?.trackCount ?? favourites?.count ?? 0);

  function open(): void {
    void router.go(playlist ? { screen: 'playlist', id: playlist.id } : { screen: 'favorites' });
  }

  async function play(e: MouseEvent): Promise<void> {
    e.stopPropagation();
    if (playlist) return api.playPlaylist(playlist.id, 0, false);
    const favs = await api.libraryFavorites();
    await api.playTracks(
      favs.map((t) => t.id),
      0,
      false,
    );
  }
</script>

<div class="card">
  <button
    class="open"
    onclick={open}
    oncontextmenu={playlist ? contextMenu(() => playlistMenu(playlist), playlist.name) : undefined}
    title={name}
  >
    {#if playlist}
      <Mosaic covers={playlist.covers} radius="var(--shape-xl)" />
    {:else}
      <span class="favourites"><Icon name="favorite" fill size={56} /></span>
    {/if}
    <span class="title title-small">{name}</span>
    <span class="sub body-small">{count.toLocaleString()} song{count === 1 ? '' : 's'}</span>
  </button>
  {#if count > 0}
    <button class="play" onclick={play} aria-label="Play {name}" title="Play">
      <Icon name="play_arrow" fill size={28} />
    </button>
  {/if}
</div>

<style>
  .card {
    position: relative;
  }
  .open {
    display: grid;
    gap: 2px;
    width: 100%;
    padding: 0;
    border: none;
    background: none;
    color: var(--md-sys-color-on-surface);
    text-align: start;
    cursor: pointer;
  }
  .open :global(.mosaic),
  .favourites {
    margin-bottom: 8px;
  }
  .favourites {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    border-radius: var(--shape-xl);
    background: linear-gradient(135deg, var(--md-sys-color-primary-container), var(--md-sys-color-tertiary-container));
    color: var(--md-sys-color-on-primary-container);
  }
  .title,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    color: var(--md-sys-color-on-surface-variant);
  }
  .play {
    position: absolute;
    right: 10px;
    bottom: 58px;
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    border: none;
    border-radius: var(--shape-lg);
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.3);
    cursor: pointer;
    opacity: 0;
    scale: 0.8;
    transition:
      opacity var(--motion-effects-fast-duration) var(--motion-effects-fast),
      scale var(--motion-spatial-fast-duration) var(--motion-spatial-fast);
  }
  .card:hover .play,
  .play:focus-visible {
    opacity: 1;
    scale: 1;
  }
</style>
