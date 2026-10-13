<script lang="ts">
  import { onMount } from 'svelte';

  import { api, type Preferences } from '../../api';
  import Switch from '../../components/Switch.svelte';
  import { toasts } from '../../toasts.svelte';

  let prefs = $state<Preferences | null>(null);

  onMount(async () => {
    prefs = (await api.getSettings()).preferences;
  });

  async function update(patch: Partial<Preferences>): Promise<void> {
    if (!prefs) return;
    try {
      prefs = (await api.setPreferences({ ...prefs, ...patch })).preferences;
    } catch (e) {
      toasts.show(String(e), 'error');
    }
  }
</script>

{#if prefs}
  <div class="row">
    <div class="row-text">
      <span class="title-medium">Find lyrics online</span>
      <span class="body-medium">
        When a song has no lyrics of its own, ask LRCLIB (lrclib.net), a free lyrics database. Only the song's
        title, artist, album and length are sent; results are kept on this PC.
      </span>
    </div>
    <Switch label="Find lyrics online" checked={prefs.lyricsOnline} onchange={(v) => update({ lyricsOnline: v })} />
  </div>

  <div class="row">
    <div class="row-text">
      <span class="title-medium">Artist pictures</span>
      <span class="body-medium">
        Show pictures of the artists in your library, from Deezer (deezer.com). Only artist names are sent; the
        pictures are kept on this PC. Turning this off removes them again.
      </span>
    </div>
    <Switch
      label="Artist pictures"
      checked={prefs.artistImagesOnline}
      onchange={(v) => update({ artistImagesOnline: v })}
    />
  </div>
{:else}
  <p class="body-medium muted">Loading…</p>
{/if}

<style>
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px 24px;
    padding: 12px 0;
  }
  .row + .row {
    border-top: 1px solid var(--md-sys-color-outline-variant);
  }
  .row-text {
    display: grid;
    flex: 1;
    gap: 2px;
    min-width: 240px;
  }
  .row-text .body-medium,
  .muted {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }
</style>
