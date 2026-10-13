<script lang="ts">
  import { onMount } from 'svelte';

  import { api, type AiStatus } from '../../api';
  import Button from '../../components/Button.svelte';
  import { dialogs } from '../../dialogs.svelte';
  import { toasts } from '../../toasts.svelte';

  let status = $state<AiStatus | null>(null);
  let key = $state('');
  let saving = $state(false);

  onMount(() => void refresh());

  async function refresh(): Promise<void> {
    try {
      status = await api.aiStatus();
    } catch (e) {
      toasts.show(String(e), 'error');
    }
  }

  async function save(): Promise<void> {
    saving = true;
    try {
      await api.aiSetKey(key);
      key = '';
      toasts.show('API key saved');
      await refresh();
    } catch (e) {
      toasts.show(String(e), 'error');
    } finally {
      saving = false;
    }
  }

  async function remove(): Promise<void> {
    if (!(await api.confirm('Remove the API key from this PC?', 'AI playlists'))) return;
    await api.aiRemoveKey();
    await refresh();
  }
</script>

{#if status}
  <div class="row column">
    <div class="row-text">
      <span class="title-medium">Anthropic API key</span>
      <span class="body-medium">
        AI playlists use your own key from console.anthropic.com, billed to your account. It is kept in Windows
        Credential Manager, never in the app's files. Each playlist sends the titles, artists, albums, years and
        genres of {status.sentSongs.toLocaleString()} songs to Anthropic{status.sentSongs < status.songs
          ? ` (a sample of your ${status.songs.toLocaleString()})`
          : ''}: about ${status.estimateUsd < 0.01 ? '0.01' : status.estimateUsd.toFixed(2)} each.
      </span>
    </div>
    {#if status.hasKey}
      <div class="inline">
        <span class="body-large saved">A key is saved.</span>
        <Button variant="tonal" icon="auto_awesome" onclick={() => (dialogs.ai = true)}>Create a playlist</Button>
        <Button variant="text" icon="delete" onclick={remove}>Remove</Button>
      </div>
    {:else}
      <form
        class="inline"
        onsubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        <input
          class="body-large"
          type="password"
          autocomplete="off"
          spellcheck="false"
          placeholder="sk-ant-…"
          aria-label="Anthropic API key"
          bind:value={key}
        />
        <Button variant="filled" type="submit" disabled={saving || !key.trim()}>Save</Button>
      </form>
    {/if}
  </div>
{:else}
  <p class="body-medium muted">Loading…</p>
{/if}

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px 0;
  }
  .row-text {
    display: grid;
    gap: 2px;
  }
  .row-text .body-medium,
  .muted {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .inline {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
  }
  .saved {
    margin-right: auto;
  }
  input {
    flex: 1;
    min-width: 220px;
    height: 48px;
    padding: 0 14px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: var(--shape-xs);
    background: none;
    color: var(--md-sys-color-on-surface);
  }
  input:focus {
    border-color: var(--md-sys-color-primary);
    outline: 1px solid var(--md-sys-color-primary);
  }
</style>
