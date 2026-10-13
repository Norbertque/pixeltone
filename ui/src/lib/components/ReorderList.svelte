<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount, tick } from 'svelte';

  import { clampTop, displacement, dropIndex, edgeScroll, type HandleProps } from '../queue/drag';
  import { scrollParent } from '../virtual/window';
  import VirtualList from './VirtualList.svelte';

  interface Props {
    count: number;
    rowHeight: number;
    label: string;
    /** Row `index`; spread `handle` onto the element that drags it. */
    row: Snippet<[number, HandleProps]>;
    /** What follows the pointer while row `index` is dragged. */
    ghost: Snippet<[number]>;
    /** A stable key per row (keeps keyboard focus on a moved row). */
    keyOf: (index: number) => string | number | undefined;
    /** Read out after a move to `index`. */
    describe?: (index: number) => string;
    onmove: (from: number, to: number) => void;
    onrange?: (start: number, end: number) => void;
  }
  let { count, rowHeight, label, row, ghost, keyOf, describe, onmove, onrange }: Props = $props();

  interface Drag {
    from: number;
    to: number;
    /** Top of the dragged row, px from the list's top. */
    top: number;
    /** Where the row was grabbed, px from its top. */
    grab: number;
    startY: number;
    pointerY: number;
    active: boolean;
  }
  let drag = $state<Drag | null>(null);
  let listEl: HTMLDivElement | undefined = $state();
  let frame = 0;
  let announcement = $state('');

  // The grabbed row stays hidden in place while a floating copy follows the pointer: the
  // virtual list may stop rendering the original once auto-scroll has carried it away.
  function shift(i: number): number {
    const d = drag;
    return d?.active ? displacement(i, d.from, d.to, rowHeight) : 0;
  }

  function follow(clientY: number): void {
    if (!drag || !listEl) return;
    const top = clampTop(clientY - listEl.getBoundingClientRect().top - drag.grab, rowHeight, count);
    drag = { ...drag, top, to: dropIndex(top, rowHeight, count), pointerY: clientY };
  }

  /** Scrolls while the pointer is held near the top or bottom edge. */
  function autoScroll(): void {
    frame = 0;
    const scroller = listEl ? scrollParent(listEl) : null;
    if (!drag?.active || !scroller) return;
    const r = scroller.getBoundingClientRect();
    const speed = edgeScroll(drag.pointerY, r.top, r.bottom);
    if (speed !== 0) {
      scroller.scrollTop += speed;
      follow(drag.pointerY);
    }
    frame = requestAnimationFrame(autoScroll);
  }

  // Window-level listeners: in a virtual list the grabbed row may be re-rendered mid-drag.
  function start(e: PointerEvent, index: number): void {
    if (e.button !== 0 || !listEl) return;
    e.preventDefault();
    drag = {
      from: index,
      to: index,
      top: index * rowHeight,
      grab: e.clientY - listEl.getBoundingClientRect().top - index * rowHeight,
      startY: e.clientY,
      pointerY: e.clientY,
      active: false,
    };
    window.addEventListener('pointermove', onpointermove);
    window.addEventListener('pointerup', onpointerup);
    window.addEventListener('pointercancel', onpointercancel);
  }

  function onpointermove(e: PointerEvent): void {
    if (!drag) return;
    if (!drag.active) {
      // A small threshold keeps a plain click on the handle from starting a drag.
      if (Math.abs(e.clientY - drag.startY) < 4) return;
      drag = { ...drag, active: true };
      frame = requestAnimationFrame(autoScroll);
    }
    follow(e.clientY);
  }

  function end(commit: boolean): void {
    window.removeEventListener('pointermove', onpointermove);
    window.removeEventListener('pointerup', onpointerup);
    window.removeEventListener('pointercancel', onpointercancel);
    cancelAnimationFrame(frame);
    const d = drag;
    drag = null;
    if (commit && d?.active && d.to !== d.from) move(d.from, d.to);
  }
  const onpointerup = (): void => end(true);
  const onpointercancel = (): void => end(false);

  function move(from: number, to: number): void {
    onmove(from, to);
    if (describe) announcement = describe(to);
  }

  /** The handle's keyboard alternative to dragging: arrow keys move the row. */
  async function key(e: KeyboardEvent, index: number): Promise<void> {
    const delta = e.key === 'ArrowUp' ? -1 : e.key === 'ArrowDown' ? 1 : 0;
    if (delta === 0) return;
    e.preventDefault();
    const to = index + delta;
    if (to < 0 || to >= count) return;
    const k = keyOf(index);
    move(index, to);
    await tick();
    if (k !== undefined) listEl?.querySelector<HTMLElement>(`[data-handle="${CSS.escape(String(k))}"]`)?.focus();
  }

  function handle(index: number): HandleProps {
    return {
      'data-handle': String(keyOf(index) ?? index),
      onpointerdown: (e) => start(e, index),
      onkeydown: (e) => void key(e, index),
    };
  }

  onMount(() => () => end(false));
</script>

{#snippet slot(i: number)}
  <div class={['slot', { ghosted: drag?.active && drag.from === i }]}>
    {@render row(i, handle(i))}
  </div>
{/snippet}

<div bind:this={listEl} class={['reorder', { dragging: drag?.active }]}>
  <VirtualList {count} {rowHeight} {label} row={slot} {shift} animate={drag?.active ?? false} {onrange} />
  {#if drag?.active}
    <div class="ghost" style:transform="translateY({drag.top}px)" style:height="{rowHeight}px" aria-hidden="true">
      {@render ghost(drag.from)}
    </div>
  {/if}
  <p class="visually-hidden" aria-live="polite">{announcement}</p>
</div>

<style>
  .reorder {
    position: relative;
  }
  .slot {
    height: 100%;
  }
  .ghosted {
    visibility: hidden;
  }
  .ghost {
    position: absolute;
    top: 0;
    right: 0;
    left: 0;
    z-index: 1;
    border-radius: var(--shape-md);
    background: var(--md-sys-color-surface-container-highest);
    box-shadow: 0 6px 16px rgb(0 0 0 / 0.25);
    pointer-events: none;
  }
  .dragging :global([data-handle]) {
    cursor: grabbing;
  }
</style>
