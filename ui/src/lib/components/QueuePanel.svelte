<script lang="ts">
  import { onMount } from 'svelte';

  import { api, type QueueEntry } from '../api';
  import { PagedRows } from '../library.svelte';
  import { artworkUrl, formatTime, player } from '../player.svelte';
  import type { HandleProps } from '../queue/drag';
  import { toasts } from '../toasts.svelte';
  import Artwork from './Artwork.svelte';
  import Button from './Button.svelte';
  import EmptyState from './EmptyState.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import ReorderList from './ReorderList.svelte';

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

  let scrollEl: HTMLDivElement | undefined = $state();

  function move(from: number, to: number): void {
    const id = pager.rows[from]?.id;
    if (id === undefined) return;
    pager.move(from, to);
    api.queueMove(id, to).catch((e) => toasts.show(String(e), 'error'));
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
  });
</script>

{#snippet row(i: number, handle: HandleProps)}
  {@const entry = pager.rows[i]}
  {#if entry}
    {@const current = entry.id === player.currentId}
    {@const played = player.queueIndex !== null && i < player.queueIndex}
    <div class={['qrow', { current, played }]}>
      <button type="button" class="handle" aria-label="Move {entry.title} (up and down arrow keys)" title="Drag to reorder" {...handle}>
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

{#snippet ghost(i: number)}
  {@const entry = pager.rows[i]}
  {#if entry}
    <div class="qrow lifted">
      <span class="handle"><Icon name="drag_indicator" size={20} /></span>
      <span class="main">
        <Artwork src={artworkUrl(entry.coverKey, 300)} size="40px" radius="var(--shape-sm)" />
        <span class="text">
          <span class="title title-small">{entry.title}</span>
          <span class="sub body-small">{entry.artist ?? entry.album ?? ''}</span>
        </span>
      </span>
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
      <ReorderList
        {count}
        rowHeight={ROW}
        label="Queue"
        {row}
        {ghost}
        keyOf={(i) => pager.rows[i]?.id}
        describe={(i) => `${pager.rows[i]?.title ?? 'Song'} moved to position ${i + 1} of ${count}`}
        onmove={move}
        onrange={(s, e) => pager.ensure(s, e)}
      />
    {/if}
  </div>
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
  .qrow.lifted {
    background: none;
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
