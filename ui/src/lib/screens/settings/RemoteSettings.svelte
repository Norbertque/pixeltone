<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import { api, type Preferences, type RemoteStatus } from '../../api';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Switch from '../../components/Switch.svelte';
  import { toasts } from '../../toasts.svelte';

  let prefs = $state<Preferences | null>(null);
  let status = $state<RemoteStatus | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;

  onMount(async () => {
    prefs = (await api.getSettings()).preferences;
    await refresh();
  });
  onDestroy(() => clearInterval(timer));

  async function refresh(): Promise<void> {
    try {
      status = await api.remoteStatus();
    } catch (e) {
      toasts.show(String(e), 'error');
    }
    // While a code is on show, watch for the phone to use it.
    clearInterval(timer);
    if (status?.code) timer = setInterval(() => void refresh(), 2000);
  }

  async function toggle(on: boolean): Promise<void> {
    if (!prefs) return;
    try {
      prefs = (await api.setPreferences({ ...prefs, remoteEnabled: on })).preferences;
      await refresh();
    } catch (e) {
      toasts.show(String(e), 'error');
    }
  }

  async function run(action: () => Promise<RemoteStatus>): Promise<void> {
    try {
      status = await action();
      await refresh();
    } catch (e) {
      toasts.show(String(e), 'error');
    }
  }

  function grouped(code: string): string {
    return `${code.slice(0, 3)} ${code.slice(3)}`;
  }

  function ago(seconds: number): string {
    const s = Date.now() / 1000 - seconds;
    if (s < 120) return 'just now';
    if (s < 3600 * 2) return `${Math.round(s / 60)} min ago`;
    if (s < 86_400 * 2) return `${Math.round(s / 3600)} h ago`;
    return `${Math.round(s / 86_400)} days ago`;
  }
</script>

{#if prefs}
  <div class="row">
    <div class="row-text">
      <span class="title-medium">Control from your phone</span>
      <span class="body-medium">
        Pixeltone for Android on a phone on the same Wi-Fi can play its music on this PC and control it, like a remote. Phones
        are paired once with a code; the connection is encrypted. Windows may ask whether Pixeltone can use private
        networks: allow it.
      </span>
    </div>
    <Switch label="Control from your phone" checked={prefs.remoteEnabled} onchange={(v) => void toggle(v)} />
  </div>

  {#if prefs.remoteEnabled && status}
    {#if !status.running}
      <p class="banner body-medium" role="alert">
        <Icon name="error" size={20} /> Remote control couldn't start. Another program may be using the network port.
      </p>
    {:else}
      <div class="row">
        <div class="row-text">
          <span class="title-medium">This PC</span>
          <span class="body-medium">
            Phones list it as <strong>{status.name}</strong>{#if status.address}. If one can't find it, enter
            <strong class="mono">{status.address}</strong>{/if}.
          </span>
        </div>
        {#if !status.code}
          <Button variant="tonal" icon="add_link" onclick={() => void run(api.remotePair)}>Pair a phone</Button>
        {/if}
      </div>

      {#if status.code}
        <div class="pairing" aria-live="polite">
          <span class="label-large">Enter this code on the phone</span>
          <span class="code display-medium">{grouped(status.code)}</span>
          <span class="body-small muted">
            Works once, for {Math.floor(status.codeSeconds / 60)}:{String(status.codeSeconds % 60).padStart(2, '0')}
          </span>
          <Button variant="text" onclick={() => void run(api.remoteStopPairing)}>Cancel</Button>
        </div>
      {/if}

      {#if status.devices.length}
        <div class="row column">
          <span class="title-medium">Paired phones</span>
          <ul class="devices">
            {#each status.devices as d (d.id)}
              <li>
                <Icon name="smartphone" />
                <span class="device-text">
                  <span class="body-large">{d.name}</span>
                  <span class="body-small muted">Last used {ago(d.lastSeen)}</span>
                </span>
                <IconButton
                  icon="link_off"
                  label="Unpair {d.name}"
                  onclick={() => void run(() => api.remoteForget(d.id))}
                />
              </li>
            {/each}
          </ul>
        </div>
      {/if}
    {/if}
  {/if}
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
  .row + .row,
  .pairing + .row {
    border-top: 1px solid var(--md-sys-color-outline-variant);
  }
  .row.column {
    flex-direction: column;
    align-items: stretch;
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
  strong {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }
  .mono {
    font-variant-numeric: tabular-nums;
  }
  .pairing {
    display: grid;
    justify-items: center;
    gap: 6px;
    margin: 4px 0 12px;
    padding: 20px;
    border-radius: var(--shape-lg);
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }
  .pairing .muted {
    color: inherit;
    opacity: 0.8;
  }
  .code {
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.08em;
  }
  .devices {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .devices li {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 12px;
    padding: 4px 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .device-text {
    display: grid;
    color: var(--md-sys-color-on-surface);
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    padding: 12px 16px;
    border-radius: var(--shape-md);
    background: var(--md-sys-color-error-container);
    color: var(--md-sys-color-on-error-container);
  }
</style>
