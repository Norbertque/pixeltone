<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';

  import { gridColumns, scrollParent, visibleRange } from '../virtual/window';

  interface Props {
    count: number;
    /** Smallest width an item may have; the grid fits as many columns as it can. */
    minItemWidth?: number;
    gap?: number;
    /** Height below the square artwork (title, subtitle). */
    textHeight?: number;
    item: Snippet<[number, number]>;
    onrange?: (start: number, end: number) => void;
    label?: string;
  }
  let { count, minItemWidth = 168, gap = 20, textHeight = 60, item, onrange, label }: Props = $props();

  let el: HTMLDivElement | undefined = $state();
  let width = $state(0);
  let firstRow = $state(0);
  let lastRow = $state(0);

  const layout = $derived(gridColumns(width, minItemWidth, gap));
  const rowHeight = $derived(layout.itemWidth + textHeight + gap);
  const rows = $derived(Math.ceil(count / layout.cols));

  function update(): void {
    if (!el || rowHeight <= 0) return;
    const scroller = scrollParent(el);
    const viewTop = scroller?.getBoundingClientRect().top ?? 0;
    const viewHeight = scroller?.clientHeight ?? window.innerHeight;
    const listTop = el.getBoundingClientRect().top - viewTop;
    const [s, e] = visibleRange(listTop, viewHeight, rowHeight, rows, 2);
    if (s !== firstRow || e !== lastRow) {
      firstRow = s;
      lastRow = e;
      onrange?.(s * layout.cols, Math.min(count, e * layout.cols));
    }
  }

  onMount(() => {
    const scroller = el ? scrollParent(el) : null;
    const target: HTMLElement | Window = scroller ?? window;
    target.addEventListener('scroll', update, { passive: true });
    const ro = new ResizeObserver(update);
    if (scroller) ro.observe(scroller);
    update();
    return () => {
      target.removeEventListener('scroll', update);
      ro.disconnect();
    };
  });

  $effect(() => {
    void [count, rowHeight, layout.cols];
    update();
  });

  const indices = $derived.by(() => {
    const out: number[] = [];
    for (let i = firstRow * layout.cols; i < Math.min(count, lastRow * layout.cols); i++) out.push(i);
    return out;
  });
</script>

<div
  bind:this={el}
  bind:clientWidth={width}
  class="vgrid"
  style:height="{Math.max(0, rows * rowHeight - gap)}px"
  role="list"
  aria-label={label}
>
  {#each indices as i (i)}
    <div
      class="vitem"
      role="listitem"
      style:width="{layout.itemWidth}px"
      style:transform="translate({(i % layout.cols) * (layout.itemWidth + gap)}px, {Math.floor(i / layout.cols) *
        rowHeight}px)"
    >
      {@render item(i, layout.itemWidth)}
    </div>
  {/each}
</div>

<style>
  .vgrid {
    position: relative;
    contain: layout paint;
  }
  .vitem {
    position: absolute;
    top: 0;
    left: 0;
  }
</style>
