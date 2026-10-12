<script lang="ts">
  import type { TrackRow } from '../api';
  import { contextMenu, menu } from '../menu.svelte';
  import { trackMenu } from '../menus';
  import { coverOf, formatShort, formatTime, player } from '../player.svelte';
  import Artwork from './Artwork.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    track: TrackRow | undefined;
    onplay: () => void;
    /** Show the track number instead of the cover (album pages). */
    numbered?: boolean;
    showAlbum?: boolean;
  }
  let { track, onplay, numbered = false, showAlbum = true }: Props = $props();

  const current = $derived(track !== undefined && player.track?.path === track.path);
  const subtitle = $derived(
    track ? [track.artist || 'Unknown artist', showAlbum ? track.album : null].filter(Boolean).join(' · ') : '',
  );
</script>

{#if track}
  <div class={['track', { current }]}>
    <button class="main" onclick={onplay} oncontextmenu={contextMenu(() => trackMenu(track), track.title)} title={track.path}>
      <span class="lead">
        {#if current}
          <span class="eq" aria-label="Now playing"><Icon name={player.playing ? 'graphic_eq' : 'pause'} size={20} /></span>
        {:else if numbered}
          <span class="no label-large">{track.trackNo ?? '–'}</span>
        {:else}
          <Artwork src={coverOf(track.cover)} size="40px" radius="var(--shape-sm)" />
        {/if}
      </span>
      <span class="text">
        <span class="title title-small">{track.title}</span>
        <span class="sub body-small">{subtitle}</span>
      </span>
      <span class="format label-small">{formatShort(track)}</span>
      <span class="time body-medium">{formatTime(track.durationMs)}</span>
    </button>
    <span class="more">
      <IconButton
        icon="more_vert"
        label="More options for {track.title}"
        size="xs"
        onclick={(e) => menu.show(e.currentTarget, trackMenu(track), track.title)}
      />
    </span>
  </div>
{:else}
  <div class="track placeholder" aria-hidden="true">
    <span class="lead"><span class="ph-box"></span></span>
    <span class="text"><span class="ph-line"></span><span class="ph-line short"></span></span>
  </div>
{/if}

<style>
  .track {
    container: row / inline-size;
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    height: 100%;
    border-radius: var(--shape-md);
  }
  .track:hover,
  .track:focus-within {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent);
  }
  .main {
    display: grid;
    grid-template-columns: 40px 1fr auto auto;
    align-items: center;
    gap: 16px;
    min-width: 0;
    height: 100%;
    padding: 0 8px;
    border: none;
    border-radius: var(--shape-md);
    background: none;
    color: var(--md-sys-color-on-surface);
    text-align: start;
    cursor: pointer;
  }
  .main:active {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent);
  }
  .more {
    padding-right: 8px;
    opacity: 0;
    transition: opacity var(--motion-effects-fast-duration) var(--motion-effects-fast);
  }
  .track:hover .more,
  .track:focus-within .more {
    opacity: 1;
  }
  .current .title {
    color: var(--md-sys-color-primary);
  }
  .lead {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
  }
  .no {
    color: var(--md-sys-color-on-surface-variant);
    font-variant-numeric: tabular-nums;
  }
  .eq {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: var(--shape-sm);
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }
  .text {
    display: grid;
    min-width: 0;
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
  .format {
    padding: 2px 8px;
    border-radius: var(--shape-xs);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface-variant);
    white-space: nowrap;
  }
  .time {
    min-width: 3.5em;
    color: var(--md-sys-color-on-surface-variant);
    font-variant-numeric: tabular-nums;
    text-align: end;
  }
  @container row (width < 520px) {
    .format {
      display: none;
    }
  }

  .placeholder {
    grid-template-columns: 40px 1fr;
    gap: 16px;
    padding: 0 8px;
  }
  .placeholder:hover {
    background: none;
  }
  .ph-box,
  .ph-line {
    display: block;
    border-radius: var(--shape-xs);
    background: var(--md-sys-color-surface-container-high);
  }
  .ph-box {
    width: 40px;
    height: 40px;
  }
  .ph-line {
    width: 40%;
    height: 12px;
    margin: 3px 0;
  }
  .ph-line.short {
    width: 25%;
  }
</style>
