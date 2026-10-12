<script lang="ts">
  import { api, type AlbumRow } from '../api';
  import { playAll } from '../library.svelte';
  import { contextMenu } from '../menu.svelte';
  import { albumMenu } from '../menus';
  import { coverOf } from '../player.svelte';
  import { router } from '../router.svelte';
  import Artwork from './Artwork.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    album: AlbumRow | undefined;
    size?: number;
  }
  let { album, size = 180 }: Props = $props();

  async function play(e: MouseEvent): Promise<void> {
    e.stopPropagation();
    if (!album) return;
    const detail = await api.libraryAlbum(album.id);
    if (detail) playAll(detail.tracks);
  }
</script>

{#if album}
  <div class="card">
    <button
      class="open"
      onclick={() => router.go({ screen: 'album', id: album.id })}
      oncontextmenu={contextMenu(() => albumMenu(album), album.title || 'Album')}
      title="{album.title} — {album.artist}"
    >
      <Artwork src={coverOf(album.cover, size > 300 ? 1000 : 300)} size="100%" radius="var(--shape-xl)" />
      <span class="title title-small">{album.title || 'Unknown album'}</span>
      <span class="artist body-small">{album.artist || 'Unknown artist'}</span>
    </button>
    <button class="play" onclick={play} aria-label="Play {album.title}" title="Play">
      <Icon name="play_arrow" fill size={28} />
    </button>
  </div>
{:else}
  <div class="card placeholder" aria-hidden="true">
    <div class="ph-art"></div>
  </div>
{/if}

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
  .open :global(.art) {
    margin-bottom: 8px;
    transition: border-radius var(--motion-spatial-default-duration) var(--motion-spatial-default);
  }
  .open:hover :global(.art) {
    border-radius: var(--shape-2xl) !important;
  }
  .title,
  .artist {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .artist {
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
      scale var(--motion-spatial-fast-duration) var(--motion-spatial-fast),
      border-radius var(--motion-spatial-fast-duration) var(--motion-spatial-fast);
  }
  .card:hover .play,
  .play:focus-visible {
    opacity: 1;
    scale: 1;
  }
  .play:active {
    border-radius: var(--shape-full);
  }
  .ph-art {
    aspect-ratio: 1;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-high);
  }
</style>
