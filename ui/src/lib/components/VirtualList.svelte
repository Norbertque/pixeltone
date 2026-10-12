<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';

  import { scrollParent, visibleRange } from '../virtual/window';

  interface Props {
    count: number;
    rowHeight: number;
    overscan?: number;
    /** Renders row `index`. */
    row: Snippet<[number]>;
    /** Called when the rendered range changes (to fetch data). */
    onrange?: (start: number, end: number) => void;
    label?: string;
    /** Extra vertical offset of row `index` in px (drag-to-reorder). */
    shift?: (index: number) => number;
    /** Animate rows sliding into new offsets. */
    animate?: boolean;
  }
  let { count, rowHeight, overscan = 8, row, onrange, label, shift, animate = false }: Props = $props();

  let el: HTMLDivElement | undefined = $state();
  let start = $state(0);
  let end = $state(0);

  function update(): void {
    if (!el) return;
    const scroller = scrollParent(el);
    const viewTop = scroller?.getBoundingClientRect().top ?? 0;
    const viewHeight = scroller?.clientHeight ?? window.innerHeight;
    const listTop = el.getBoundingClientRect().top - viewTop;
    const [s, e] = visibleRange(listTop, viewHeight, rowHeight, count, overscan);
    if (s !== start || e !== end) {
      start = s;
      end = e;
      onrange?.(s, e);
    }
  }

  onMount(() => {
    const scroller = el ? scrollParent(el) : null;
    const target: HTMLElement | Window = scroller ?? window;
    target.addEventListener('scroll', update, { passive: true });
    const ro = new ResizeObserver(update);
    if (el) ro.observe(el);
    if (scroller) ro.observe(scroller);
    update();
    return () => {
      target.removeEventListener('scroll', update);
      ro.disconnect();
    };
  });

  $effect(() => {
    void [count, rowHeight];
    update();
  });

  const indices = $derived(Array.from({ length: Math.max(0, end - start) }, (_, i) => start + i));
</script>

<div bind:this={el} class={['vlist', { animate }]} style:height="{count * rowHeight}px" role="list" aria-label={label}>
  {#each indices as i (i)}
    <div
      class="vrow"
      role="listitem"
      style:transform="translateY({i * rowHeight + (shift?.(i) ?? 0)}px)"
      style:height="{rowHeight}px"
    >
      {@render row(i)}
    </div>
  {/each}
</div>

<style>
  .vlist {
    position: relative;
    contain: layout paint;
  }
  .vrow {
    position: absolute;
    top: 0;
    right: 0;
    left: 0;
  }
  .animate .vrow {
    transition: transform var(--motion-spatial-fast-duration) var(--motion-spatial-fast);
  }
</style>
