<script lang="ts">
  import type { ArtistRow } from '../api';
  import { coverOf } from '../player.svelte';
  import { router } from '../router.svelte';
  import Artwork from './Artwork.svelte';

  interface Props {
    artist: ArtistRow | undefined;
  }
  let { artist }: Props = $props();
</script>

{#if artist}
  <button class="artist" onclick={() => router.go({ screen: 'artist', id: artist.id })}>
    <Artwork src={coverOf(artist.cover)} size="48px" radius="var(--shape-full)" />
    <span class="text">
      <span class="name title-small">{artist.name}</span>
      <span class="sub body-small">
        {artist.albumCount} album{artist.albumCount === 1 ? '' : 's'} · {artist.trackCount} song{artist.trackCount === 1
          ? ''
          : 's'}
      </span>
    </span>
  </button>
{:else}
  <div class="artist" aria-hidden="true"><span class="ph"></span></div>
{/if}

<style>
  .artist {
    display: flex;
    align-items: center;
    gap: 16px;
    width: 100%;
    height: 100%;
    padding: 0 12px 0 8px;
    border: none;
    border-radius: var(--shape-md);
    background: none;
    color: var(--md-sys-color-on-surface);
    text-align: start;
    cursor: pointer;
  }
  .artist:hover {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent);
  }
  .text {
    display: grid;
    min-width: 0;
  }
  .name,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    color: var(--md-sys-color-on-surface-variant);
  }
  .ph {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: var(--md-sys-color-surface-container-high);
  }
</style>
