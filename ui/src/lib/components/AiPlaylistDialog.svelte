<script lang="ts">
  import { tick } from 'svelte';

  import { api, type AiStatus } from '../api';
  import { dialogs } from '../dialogs.svelte';
  import { router } from '../router.svelte';
  import { toasts } from '../toasts.svelte';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';
  import SegmentedButtons from './SegmentedButtons.svelte';

  const SIZES = [10, 25, 50, 100].map((n) => ({ value: n, label: String(n) }));
  const IDEAS = ['Upbeat songs for a morning run', 'Calm late-night listening', 'Loud and fast, no ballads', 'A tour of my oldest albums'];

  let dialog: HTMLDialogElement | undefined = $state();
  let field: HTMLTextAreaElement | undefined = $state();
  let prompt = $state('');
  let songs = $state(25);
  let status = $state<AiStatus | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!dialog) return;
    if (dialogs.ai && !dialog.open) {
      error = null;
      dialog.showModal();
      void api
        .aiStatus()
        .then((s) => (status = s))
        .catch((e: unknown) => (error = String(e)));
      void tick().then(() => field?.focus());
    } else if (!dialogs.ai && dialog.open) {
      dialog.close();
    }
  });

  function cost(usd: number): string {
    return usd < 0.01 ? 'under 1 cent' : `about $${usd < 1 ? usd.toFixed(2) : usd.toFixed(1)}`;
  }

  async function create(): Promise<void> {
    if (!prompt.trim() || busy) return;
    busy = true;
    error = null;
    try {
      const made = await api.aiPlaylist(prompt.trim(), songs);
      dialogs.ai = false;
      prompt = '';
      toasts.show(`“${made.name}”: ${made.songs} songs. ${made.description}`);
      await router.go({ screen: 'playlist', id: made.playlistId });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="dialog"
  aria-labelledby="ai-title"
  onclose={() => {
    if (dialogs.ai) dialogs.ai = false;
  }}
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void create();
    }}
  >
    <h2 id="ai-title" class="headline-small"><Icon name="auto_awesome" /> Create a playlist with AI</h2>

    {#if status && !status.hasKey}
      <p class="banner body-medium">
        <Icon name="key" size={20} />
        <span>This uses your own Anthropic API key. Add it in Settings → AI playlists first.</span>
      </p>
      <div class="actions">
        <Button variant="text" onclick={() => (dialogs.ai = false)}>Close</Button>
        <Button
          variant="filled"
          onclick={() => {
            dialogs.ai = false;
            void router.go('settings');
          }}>Open Settings</Button
        >
      </div>
    {:else}
      <label class="field">
        <span class="label-medium">What should it be like?</span>
        <textarea
          bind:this={field}
          bind:value={prompt}
          class="body-large"
          rows="3"
          maxlength="500"
          placeholder={IDEAS[0]}
          disabled={busy}
          onkeydown={(e) => {
            if (e.key === 'Enter' && (e.ctrlKey || !e.shiftKey)) {
              e.preventDefault();
              void create();
            }
          }}
        ></textarea>
      </label>
      <div class="ideas">
        {#each IDEAS.slice(1) as idea (idea)}
          <button type="button" class="chip label-large" disabled={busy} onclick={() => (prompt = idea)}>{idea}</button>
        {/each}
      </div>
      <div class="size">
        <span class="label-large">Songs</span>
        <SegmentedButtons label="Number of songs" options={SIZES} value={songs} onchange={(v) => (songs = v)} />
      </div>

      {#if error}
        <p class="banner error body-medium" role="alert"><Icon name="error" size={20} /> <span>{error}</span></p>
      {/if}

      <p class="note body-small">
        {#if status}
          Claude picks from {status.sentSongs.toLocaleString()} of your songs: their titles, artists, albums,
          years and genres are sent to Anthropic. Billed to your key, {cost(status.estimateUsd)} per playlist.
        {:else}
          Checking…
        {/if}
      </p>

      <div class="actions">
        <Button variant="text" onclick={() => (dialogs.ai = false)} disabled={busy}>Cancel</Button>
        <Button variant="filled" type="submit" icon={busy ? undefined : 'auto_awesome'} disabled={busy || !prompt.trim() || !status}>
          {busy ? 'Picking songs…' : 'Create'}
        </Button>
      </div>
    {/if}
  </form>
</dialog>

<style>
  .dialog {
    width: min(560px, calc(100vw - 32px));
    padding: 24px;
    border: none;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
  }
  .dialog::backdrop {
    background: rgb(0 0 0 / 0.32);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 20px;
  }
  h2 :global(.icon) {
    color: var(--md-sys-color-primary);
  }
  .field {
    display: grid;
    gap: 4px;
    color: var(--md-sys-color-primary);
  }
  textarea {
    resize: vertical;
    min-height: 88px;
    padding: 12px 14px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: var(--shape-xs);
    background: none;
    color: var(--md-sys-color-on-surface);
    font: inherit;
  }
  textarea:focus {
    border-color: var(--md-sys-color-primary);
    outline: 1px solid var(--md-sys-color-primary);
  }
  .ideas {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }
  .chip {
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: var(--shape-sm);
    background: none;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
  }
  .chip:hover:not(:disabled) {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent);
  }
  .size {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 20px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 16px 0 0;
    padding: 12px 16px;
    border-radius: var(--shape-md);
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .banner.error {
    background: var(--md-sys-color-error-container);
    color: var(--md-sys-color-on-error-container);
  }
  .note {
    margin: 16px 0 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 24px;
  }
</style>
