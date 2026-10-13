<script lang="ts">
  import { api, type PlaylistRow } from '../api';
  import { dialogs } from '../dialogs.svelte';
  import { toasts } from '../toasts.svelte';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';
  import Mosaic from './Mosaic.svelte';

  let dialog: HTMLDialogElement | undefined = $state();
  let playlists = $state<PlaylistRow[] | null>(null);
  const req = $derived(dialogs.pick);

  $effect(() => {
    if (!dialog) return;
    if (req && !dialog.open) {
      playlists = null;
      void api.libraryPlaylists().then((p) => (playlists = p));
      dialog.showModal();
    } else if (!req && dialog.open) {
      dialog.close();
    }
  });

  async function choose(p: PlaylistRow): Promise<void> {
    const r = dialogs.pick;
    dialogs.pick = null;
    if (!r) return;
    try {
      await api.playlistAddTracks(p.id, r.trackIds);
      toasts.show(`Added “${r.label}” to “${p.name}”`);
    } catch (e) {
      toasts.show(String(e), 'error');
    }
  }

  async function create(): Promise<void> {
    const r = dialogs.pick;
    dialogs.pick = null;
    if (!r) return;
    const name = await dialogs.ask('New playlist', { label: 'Name', confirm: 'Create' });
    if (!name) return;
    try {
      await api.playlistCreate(name, r.trackIds);
      toasts.show(`Created “${name}” with “${r.label}”`);
    } catch (e) {
      toasts.show(String(e), 'error');
    }
  }
</script>

<dialog bind:this={dialog} class="dialog" aria-labelledby="picker-title" onclose={() => (dialogs.pick = null)}>
  <h2 id="picker-title" class="headline-small">Add to playlist</h2>
  {#if req}<p class="what body-medium">{req.label}</p>{/if}
  <div class="list scroller">
    <button type="button" class="row new" onclick={create}>
      <span class="thumb"><Icon name="add" /></span>
      <span class="title-medium">New playlist…</span>
    </button>
    {#if playlists === null}
      <p class="body-medium muted">Loading…</p>
    {:else}
      {#each playlists as p (p.id)}
        <button type="button" class="row" onclick={() => choose(p)}>
          <Mosaic covers={p.covers} size="48px" />
          <span class="text">
            <span class="title-medium">{p.name}</span>
            <span class="body-small muted">{p.trackCount} song{p.trackCount === 1 ? '' : 's'}</span>
          </span>
        </button>
      {/each}
    {/if}
  </div>
  <div class="actions">
    <Button variant="text" onclick={() => (dialogs.pick = null)}>Cancel</Button>
  </div>
</dialog>

<style>
  .dialog {
    width: min(440px, calc(100vw - 32px));
    max-height: min(640px, calc(100vh - 48px));
    padding: 24px 16px 16px;
    border: none;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
  }
  .dialog[open] {
    display: grid;
    grid-template-rows: auto auto 1fr auto;
  }
  .dialog::backdrop {
    background: rgb(0 0 0 / 0.32);
  }
  h2 {
    margin: 0 8px;
  }
  .what {
    overflow: hidden;
    margin: 4px 8px 12px;
    color: var(--md-sys-color-on-surface-variant);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .list {
    display: grid;
    align-content: start;
    gap: 2px;
    min-height: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 64px;
    padding: 8px;
    border: none;
    border-radius: var(--shape-md);
    background: none;
    color: inherit;
    text-align: start;
    cursor: pointer;
  }
  .row:hover,
  .row:focus-visible {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent);
    outline: none;
  }
  .thumb {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: var(--shape-md);
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }
  .text {
    display: grid;
  }
  .muted {
    color: var(--md-sys-color-on-surface-variant);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 8px;
  }
</style>
