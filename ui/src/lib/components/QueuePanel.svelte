<script lang="ts">
  import { onMount, tick } from 'svelte';

  import { api, type QueueEntry } from '../api';
  import { PagedRows } from '../library.svelte';
  import { artworkUrl, formatTime, player } from '../player.svelte';
  import { clampTop, displacement, dropIndex, edgeScroll } from '../queue/drag';
  import { toasts } from '../toasts.svelte';
  import Artwork from './Artwork.svelte';
  import Button from './Button.svelte';
  import EmptyState from './EmptyState.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import VirtualList from './VirtualList.svelte';

  interface Props {
    onclose?: () => void;
  }
  let { onclose }: Props = $props();

  const ROW = 64;

  // One pager for the panel's lifetime, refreshed in place whenever the queue changes.
  const pager = new PagedRows<QueueEntry>(player.queueLength, (offset, limit) => api.queueItems(offset, limit), 100);
  let seen = player.queueVersion;
  $effect(() => {
    const version = player.queueVersion;
    const length = player.queueLength;
    if (version !== seen) {
      seen = version;
      pager.refresh(length);
    }
  });
  const count = $derived(pager.count);

  interface Drag {
    id: number;
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
  let scrollEl: HTMLDivElement | undefined = $state();
  let frame = 0;
  let announcement = $state('');

  function shift(i: number): number {
    const d = drag;
    if (!d?.active) return 0;
    if (i === d.from) return d.top - i * ROW;
    return displacement(i, d.from, d.to, ROW);
  }

  /** Follows the pointer at `clientY`. */
  function follow(clientY: number): void {
    if (!drag || !listEl) return;
    const top = clampTop(clientY - listEl.getBoundingClientRect().top - drag.grab, ROW, count);
    drag = { ...drag, top, to: dropIndex(top, ROW, count), pointerY: clientY };
  }

  /** Scrolls while the pointer is held near the top or bottom edge. */
  function autoScroll(): void {
    frame = 0;
    if (!drag?.active || !scrollEl) return;
    const r = scrollEl.getBoundingClientRect();
    const speed = edgeScroll(drag.pointerY, r.top, r.bottom);
    if (speed !== 0) {
      scrollEl.scrollTop += speed;
      follow(drag.pointerY);
    }
    frame = requestAnimationFrame(autoScroll);
  }

  // Window-level listeners: in a virtual list the grabbed row may be re-rendered mid-drag.
  function onpointerdown(e: PointerEvent, entry: QueueEntry, index: number): void {
    if (e.button !== 0 || !listEl) return;
    e.preventDefault();
    const grab = e.clientY - listEl.getBoundingClientRect().top - index * ROW;
    drag = {
      id: entry.id,
      from: index,
      to: index,
      top: index * ROW,
      grab,
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
    if (commit && d?.active && d.to !== d.from) move(d.id, d.from, d.to);
  }
  const onpointerup = (): void => end(true);
  const onpointercancel = (): void => end(false);

  function move(id: number, from: number, to: number): void {
    const title = pager.rows[from]?.title ?? 'Song';
    pager.move(from, to);
    announcement = `${title} moved to position ${to + 1} of ${count}`;
    api.queueMove(id, to).catch((e) => toasts.show(String(e), 'error'));
  }

  /** The handle's keyboard alternative to dragging: arrow keys move the row. */
  async function onhandlekey(e: KeyboardEvent, entry: QueueEntry, index: number): Promise<void> {
    const delta = e.key === 'ArrowUp' ? -1 : e.key === 'ArrowDown' ? 1 : 0;
    if (delta === 0) return;
    e.preventDefault();
    const to = index + delta;
    if (to < 0 || to >= count) return;
    move(entry.id, index, to);
    await tick();
    listEl?.querySelector<HTMLElement>(`[data-handle="${entry.id}"]`)?.focus();
  }

  function remove(entry: QueueEntry): void {
    api.queueRemove([entry.id]).catch((e) => toasts.show(String(e), 'error'));
  }

  function clear(): void {
    api.queueClear().then(
      () => toasts.show('Cleared the queue'),
      (e) => toasts.show(String(e), 'error'),
    );
  }

  onMount(() => {
    // Open with the current track near the top.
    const i = player.queueIndex;
    if (scrollEl && i !== null) scrollEl.scrollTop = Math.max(0, (i - 1) * ROW);
    return () => end(false);
  });
</script>

{#snippet row(i: number)}
  {@const entry = pager.rows[i]}
  {#if entry}
    {@const current = entry.id === player.currentId}
    {@const played = player.queueIndex !== null && i < player.queueIndex}
    <div class={['qrow', { current, played, dragged: drag?.active && drag.id === entry.id }]}>
      <button
        type="button"
        class="handle"
        data-handle={entry.id}
        aria-label="Move {entry.title} (up and down arrow keys)"
        title="Drag to reorder"
        onpointerdown={(e) => onpointerdown(e, entry, i)}
        onkeydown={(e) => onhandlekey(e, entry, i)}
      >
        <Icon name="drag_indicator" size={20} />
      </button>
      <button type="button" class="main" onclick={() => api.queueJump(entry.id)} title={entry.path}>
        <Artwork src={artworkUrl(entry.coverKey, 300)} size="40px" radius="var(--shape-sm)" />
        <span class="text">
          <span class="title title-small">{entry.title}</span>
          <span class="sub body-small">{entry.artist ?? entry.album ?? ''}</span>
        </span>
        {#if current}
          <span class="now" aria-label="Now playing"><Icon name={player.playing ? 'graphic_eq' : 'pause'} size={20} /></span>
        {:else if entry.durationMs !== null}
          <span class="time body-small">{formatTime(entry.durationMs)}</span>
        {/if}
      </button>
      {#if current}
        <span class="slot"></span>
      {:else}
        <IconButton icon="close" label="Remove {entry.title} from the queue" size="xs" onclick={() => remove(entry)} />
      {/if}
    </div>
  {:else}
    <div class="qrow placeholder" aria-hidden="true">
      <span></span><span class="ph-box"></span><span class="ph-line"></span>
    </div>
  {/if}
{/snippet}

<aside class="queue" aria-labelledby="queue-title">
  <header class="head">
    <div class="titles">
      <h2 id="queue-title" class="title-large rounded emphasized">Queue</h2>
      <span class="count body-medium">{count.toLocaleString()} song{count === 1 ? '' : 's'}</span>
    </div>
    <IconButton
      icon="shuffle"
      label={player.shuffle ? 'Shuffle: on' : 'Shuffle: off'}
      selected={player.shuffle}
      onclick={() => player.toggleShuffle()}
    />
    <Button variant="text" icon="clear_all" disabled={count <= 1} onclick={clear}>Clear</Button>
    {#if onclose}
      <IconButton icon="close" label="Close the queue" onclick={onclose} />
    {/if}
  </header>
  <div class="body scroller" bind:this={scrollEl}>
    {#if count === 0}
      <EmptyState icon="queue_music" title="Nothing queued" body="Play something, or use “Add to queue” on any song." />
    {:else}
      <div bind:this={listEl} class:dragging={drag?.active}>
        <VirtualList
          {count}
          rowHeight={ROW}
          label="Queue"
          {row}
          {shift}
          animate={drag?.active ?? false}
          onrange={(s, e) => pager.ensure(s, e)}
        />
      </div>
    {/if}
  </div>
  <p class="visually-hidden" aria-live="polite">{announcement}</p>
</aside>

<style>
  .queue {
    display: grid;
    grid-template-rows: auto 1fr;
    min-width: 0;
    min-height: 0;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container);
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 12px 12px 8px 20px;
  }
  .titles {
    display: grid;
    flex: 1;
    min-width: 0;
  }
  h2 {
    margin: 0;
  }
  .count {
    color: var(--md-sys-color-on-surface-variant);
  }
  .body {
    min-height: 0;
    padding: 0 8px 12px;
  }

  .qrow {
    display: grid;
    grid-template-columns: 32px 1fr 32px;
    align-items: center;
    gap: 4px;
    height: 100%;
    padding-right: 4px;
    border-radius: var(--shape-md);
    background: var(--md-sys-color-surface-container);
  }
  .qrow:hover,
  .qrow:focus-within {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, var(--md-sys-color-surface-container));
  }
  .qrow.current {
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .qrow.played .main {
    opacity: 0.6;
  }
  .qrow.dragged {
    position: relative;
    z-index: 1;
    background: var(--md-sys-color-surface-container-highest);
    box-shadow: 0 6px 16px rgb(0 0 0 / 0.25);
  }
  .dragging :global(.vrow:has(.dragged)) {
    z-index: 1;
    transition: none;
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
  .dragging .handle {
    cursor: grabbing;
  }
  .handle:focus-visible {
    outline: 2px solid var(--md-sys-color-secondary);
  }

  .main {
    display: grid;
    grid-template-columns: 40px 1fr auto;
    align-items: center;
    gap: 12px;
    min-width: 0;
    height: 100%;
    padding: 0 8px 0 0;
    border: none;
    background: none;
    color: inherit;
    text-align: start;
    cursor: pointer;
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
  .sub,
  .time {
    color: var(--md-sys-color-on-surface-variant);
  }
  .current .sub {
    color: inherit;
    opacity: 0.8;
  }
  .time {
    font-variant-numeric: tabular-nums;
  }
  .now {
    display: grid;
    place-items: center;
    color: var(--md-sys-color-primary);
  }
  .slot {
    width: 32px;
  }

  .placeholder {
    grid-template-columns: 32px 40px 1fr;
    gap: 12px;
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
    width: 50%;
    height: 12px;
  }
</style>
