<script lang="ts">
  import { scallopPath } from '../shapes';
  import Icon from './Icon.svelte';

  interface Props {
    playing: boolean;
    onclick: () => void;
    size?: number;
    disabled?: boolean;
  }
  let { playing, onclick, size = 96, disabled = false }: Props = $props();

  // Same point count, so the browser can morph one into the other.
  const cookie = $derived(scallopPath(size, 8, 0.075));
  const circle = $derived(scallopPath(size, 8, 0));
</script>

<button
  type="button"
  class={['play', { playing }]}
  aria-label={playing ? 'Pause' : 'Play'}
  title={playing ? 'Pause' : 'Play'}
  {onclick}
  {disabled}
  style:--size="{size}px"
>
  <span class="shape" style:clip-path="path('{playing ? cookie : circle}')"></span>
  <span class="glyph"><Icon name={playing ? 'pause' : 'play_arrow'} size={Math.round(size * 0.42)} fill /></span>
</button>

<style>
  .play {
    position: relative;
    display: grid;
    place-items: center;
    width: var(--size);
    height: var(--size);
    padding: 0;
    border: none;
    background: none;
    color: var(--md-sys-color-on-primary);
    cursor: pointer;
    transition: scale var(--motion-spatial-fast-duration) var(--motion-spatial-fast);
  }
  .play:active {
    scale: 0.94;
  }
  .shape {
    position: absolute;
    inset: 0;
    background: var(--md-sys-color-primary);
    transition: clip-path var(--motion-spatial-slow-duration) var(--motion-spatial-slow);
  }
  .playing .shape {
    animation: spin 14s linear infinite;
  }
  .play:hover .shape {
    background: color-mix(in srgb, var(--md-sys-color-on-primary) 8%, var(--md-sys-color-primary));
  }
  .glyph {
    position: relative;
    display: grid;
  }
  .play:disabled {
    opacity: 0.38;
    cursor: default;
  }
  @keyframes spin {
    to {
      rotate: 1turn;
    }
  }
</style>
