<script lang="ts">
  import { coverOf } from '../player.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    /** Library cover hashes; one fills the tile, two to four make a 2×2 grid. */
    covers: readonly string[];
    size?: string;
    radius?: string;
    icon?: string;
  }
  let { covers, size = '100%', radius = 'var(--shape-md)', icon = 'queue_music' }: Props = $props();

  const cells = $derived(covers.length >= 2 ? [0, 1, 2, 3].map((i) => covers[i % covers.length]!) : covers.slice(0, 1));
</script>

<div class={['mosaic', { grid: cells.length === 4 }]} style:width={size} style:border-radius={radius} aria-hidden="true">
  {#if cells.length === 0}
    <span class="empty"><Icon name={icon} size={32} /></span>
  {:else}
    {#each cells as c, i (i)}
      <img src={coverOf(c, cells.length === 1 ? 1000 : 300)} alt="" loading="lazy" decoding="async" />
    {/each}
  {/if}
</div>

<style>
  .mosaic {
    display: grid;
    flex: none;
    aspect-ratio: 1;
    overflow: hidden;
    background: var(--md-sys-color-surface-container-highest);
  }
  .mosaic.grid {
    grid-template-columns: 1fr 1fr;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .empty {
    display: grid;
    place-items: center;
    color: var(--md-sys-color-on-surface-variant);
  }
</style>
