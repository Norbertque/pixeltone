<script lang="ts">
  import { onMount } from 'svelte';

  import { api, type Preferences } from '../../api';
  import Button from '../../components/Button.svelte';
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
      <span class="title-medium">Keep playing in the tray</span>
      <span class="body-medium">
        Closing the window hides it; the music goes on, controlled from the icon next to the clock. Quit from that
        icon's menu.
      </span>
    </div>
    <Switch label="Keep playing in the tray" checked={prefs.closeToTray} onchange={(v) => update({ closeToTray: v })} />
  </div>
  <div class="row">
    <div class="row-text">
      <span class="title-medium">Mini player</span>
      <span class="body-medium">A small window that stays on top of others. Also in the player bar and the tray menu.</span>
    </div>
    <Button variant="outlined" icon="picture_in_picture_alt" onclick={() => void api.openMiniPlayer()}>Open</Button>
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
