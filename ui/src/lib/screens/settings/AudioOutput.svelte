<script lang="ts">
  import { onMount } from 'svelte';

  import { api, type DeviceInfo, type SampleFormat } from '../../api';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import OutputBadge from '../../components/OutputBadge.svelte';
  import SegmentedButtons from '../../components/SegmentedButtons.svelte';
  import Slider from '../../components/Slider.svelte';
  import Switch from '../../components/Switch.svelte';
  import { output as store } from '../../output.svelte';
  import { player } from '../../player.svelte';
  import { toasts } from '../../toasts.svelte';
  import { formatDb, volumeIcon } from '../../volume';

  const RATES = [44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000, 705_600, 768_000];
  const CONTAINERS: { label: string; sample: SampleFormat }[] = [
    { label: '16-bit', sample: { kind: 'int', containerBits: 16, validBits: 16 } },
    { label: '24-bit', sample: { kind: 'int', containerBits: 24, validBits: 24 } },
    { label: '24-in-32', sample: { kind: 'int', containerBits: 32, validBits: 24 } },
    { label: '32-bit', sample: { kind: 'int', containerBits: 32, validBits: 32 } },
    { label: '32-bit float', sample: { kind: 'float', containerBits: 32, validBits: 32 } },
  ];

  let devices = $state<DeviceInfo[]>([]);
  let dialog: HTMLDialogElement | undefined = $state();

  const output = $derived(store.settings?.output ?? null);
  const prefs = $derived(store.prefs);
  const caps = $derived(store.caps);
  const volume = $derived(store.volume);
  const volumeDb = $derived(store.draft ?? volume?.db ?? 0);
  const hasDeviceVolume = $derived(store.hardware !== null);
  const live = $derived(player.snapshot?.output ?? null);

  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;
    void (async () => {
      await store.start();
      try {
        devices = await api.outputDevices();
      } catch (e) {
        toasts.show(`Can't list audio devices: ${e}`, 'error');
      }
      await store.probe();
    })();
    // Device notifications come in bursts (one per role and state): let them settle.
    void api
      .onDevicesChanged(() => {
        clearTimeout(timer);
        timer = setTimeout(refreshDevices, 400);
      })
      .then((u) => (disposed ? u() : (unlisten = u)));
    return () => {
      disposed = true;
      unlisten?.();
      clearTimeout(timer);
    };
  });

  async function refreshDevices(): Promise<void> {
    try {
      devices = await api.outputDevices();
    } catch {
      return;
    }
    // "Windows default" may mean a different device now.
    if (store.deviceId === null) await store.probe();
  }

  function supported(sample: SampleFormat, rate: number): boolean {
    return (
      caps?.exclusive.some(
        (c) =>
          c.channels === 2 &&
          c.sample.kind === sample.kind &&
          c.sample.containerBits === sample.containerBits &&
          c.sample.validBits === sample.validBits &&
          c.sampleRates.includes(rate),
      ) ?? false
    );
  }

  function khz(rate: number): string {
    return String(Number((rate / 1000).toFixed(1)));
  }

  function ms(frames: number, rate: number): string {
    return rate > 0 ? `${((frames / rate) * 1000).toFixed(1)} ms` : '–';
  }
</script>

{#if output && prefs}
  <div class="row">
    <div class="row-text">
      <span class="title-medium">Output device</span>
      <span class="body-medium">Changing it takes effect immediately.</span>
    </div>
    <select
      class="select body-large"
      value={store.deviceId ?? ''}
      onchange={(e) => store.update({ deviceId: e.currentTarget.value || null })}
      aria-label="Output device"
    >
      <option value="">Windows default{devices.find((d) => d.isDefault) ? ` (${devices.find((d) => d.isDefault)?.name})` : ''}</option>
      {#each devices as d (d.id)}
        <option value={d.id}>{d.name}</option>
      {/each}
    </select>
  </div>

  <div class="row">
    <div class="row-text">
      <span class="title-medium">Exclusive mode (WASAPI)</span>
      <span class="body-medium">
        Bit-perfect: the device receives your files untouched, at their own sample rate and bit depth. Other apps
        can't play through it meanwhile.
      </span>
    </div>
    <Switch label="Exclusive mode" checked={output.exclusive} onchange={(v) => store.update({ exclusive: v })} />
  </div>

  {#if output.exclusive}
    <div class="row">
      <div class="row-text">
        <span class="title-medium">Timing</span>
        <span class="body-medium">Polling is the safe choice for USB DACs on the standard Windows driver.</span>
      </div>
      <SegmentedButtons
        label="Exclusive mode timing"
        options={[
          { value: 'polling', label: 'Polling' },
          { value: 'events', label: 'Event-driven' },
        ]}
        value={output.eventDriven ? 'events' : 'polling'}
        onchange={(v) => store.update({ eventDriven: v === 'events' })}
      />
    </div>

    <div class="row">
      <div class="row-text">
        <span class="title-medium">DSD over DoP</span>
        <span class="body-medium">
          DSD files reach DACs that understand DoP untouched. Only used where the device has its own volume control
          (or at line level), because DoP can't be attenuated; otherwise DSD is converted to PCM.
        </span>
      </div>
      <Switch label="DSD over DoP" checked={output.dop} onchange={(v) => store.update({ dop: v })} />
    </div>
  {/if}

  <div class="row">
    <div class="row-text">
      <span class="title-medium">Device buffer</span>
      <span class="body-medium">Larger is more robust against dropouts; smaller reacts faster.</span>
    </div>
    <SegmentedButtons
      label="Device buffer"
      options={[20, 50, 100, 200].map((v) => ({ value: v, label: `${v} ms` }))}
      value={output.bufferMs}
      onchange={(v) => store.update({ bufferMs: v })}
    />
  </div>

  <div class="row">
    <div class="row-text">
      <span class="title-medium">Smooth pause and seek</span>
      <span class="body-medium">
        A short fade when you pause, resume, seek or skip, so it doesn't click. The music in between stays
        untouched (bit-perfect).
      </span>
    </div>
    <Switch label="Smooth pause and seek" checked={output.fades} onchange={(v) => store.update({ fades: v })} />
  </div>

  <div class="row column">
    <div class="row-text">
      <span class="title-medium">Volume</span>
      {#if !volume}
        <span class="body-medium">Checking the device…</span>
      {:else if volume.kind === 'app'}
        <span class="body-medium">
          The app's own level, applied before Windows mixes. The Windows volume applies on top.
        </span>
      {:else if volume.kind === 'device'}
        <span class="body-medium">
          This device has its own volume control (the same one as the Windows volume), so the music stays
          bit-perfect.
        </span>
      {:else if volume.kind === 'software'}
        <span class="body-medium">
          {hasDeviceVolume
            ? 'The app attenuates in 32-bit before the device; its own volume stays where it is.'
            : "This device has no volume control of its own, so the app attenuates in 32-bit before the DAC."}
          Only 0 dB is bit-perfect.
        </span>
      {:else}
        <span class="body-medium">Full scale, for a DAC feeding an amplifier with its own volume.</span>
      {/if}
    </div>

    {#if volume && volume.kind !== 'fixed'}
      <div class="volume">
        <IconButton
          icon={volumeIcon({ ...volume, db: volumeDb })}
          label={volume.muted ? 'Unmute' : 'Mute'}
          selected={volume.muted}
          onclick={() => store.toggleMute()}
        />
        <Slider
          label={volume.label}
          min={volume.min}
          max={volume.max}
          step={volume.step}
          value={volumeDb}
          valueText={formatDb(volumeDb, volume.kind)}
          oninput={(v) => store.setVolume(v)}
          onchange={(v) => store.setVolume(v)}
        />
        <span class="db label-large">{volume.muted ? 'Muted' : formatDb(volumeDb, volume.kind)}</span>
      </div>
    {/if}

    {#if output.exclusive && hasDeviceVolume}
      <div class="inline-row">
        <div class="row-text">
          <span class="title-small">Use the app's volume instead</span>
          <span class="body-medium">
            For earphones that are still too loud at the device's lowest setting. Not bit-perfect below 0 dB.
          </span>
        </div>
        <Switch
          label="Use the app's volume instead"
          checked={prefs.softwareVolume}
          onchange={(v) => store.update({}, { softwareVolume: v })}
        />
      </div>
    {/if}
    {#if output.exclusive && (!hasDeviceVolume || prefs.softwareVolume)}
      <div class="inline-row">
        <div class="row-text">
          <span class="title-small">Fixed line-level output</span>
          <span class="body-medium">Full scale, for a DAC feeding an amplifier with its own volume.</span>
        </div>
        <Switch label="Fixed line-level output" checked={prefs.lineLevel} onchange={(v) => store.update({}, { lineLevel: v })} />
      </div>
    {/if}
    {#if volume?.kind === 'software' && !prefs.loudnessAcknowledged}
      <div>
        <Button variant="text" icon="lock_open" onclick={() => dialog?.showModal()}>Unlock levels above −10 dB</Button>
      </div>
    {/if}
  </div>

  <div class="row column">
    <div class="caps-head">
      <div class="row-text">
        <span class="title-medium">Exclusive-mode formats</span>
        <span class="body-medium">What {caps?.device.name ?? 'the device'} accepts directly, stereo.</span>
      </div>
      <IconButton icon="refresh" label="Check again" onclick={() => void store.probe()} disabled={store.capsLoading} />
    </div>
    {#if caps?.exclusiveError}
      <p class="banner body-medium" role="alert"><Icon name="warning" size={20} /> {caps.exclusiveError}</p>
    {/if}
    {#if caps && caps.exclusive.length > 0}
      <div class="table-wrap scroller">
        <table class="caps label-medium">
          <thead>
            <tr>
              <th scope="col">kHz</th>
              {#each RATES as r (r)}<th scope="col">{khz(r)}</th>{/each}
            </tr>
          </thead>
          <tbody>
            {#each CONTAINERS as c (c.label)}
              <tr>
                <th scope="row">{c.label}</th>
                {#each RATES as r (r)}
                  <td class:yes={supported(c.sample, r)} aria-label={supported(c.sample, r) ? 'supported' : 'not supported'}>
                    {#if supported(c.sample, r)}<Icon name="check" size={16} />{/if}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
    {#if caps?.mixFormat}
      <p class="body-small muted">
        Shared mode mixes at {caps.mixFormat.sample.kind === 'float' ? '32-bit float' : `${caps.mixFormat.sample.validBits}-bit`},
        {khz(caps.mixFormat.sampleRate)} kHz, {caps.mixFormat.channels} channels. Device period {(
          caps.defaultPeriodUs / 1000
        ).toFixed(1)} ms (minimum {(caps.minPeriodUs / 1000).toFixed(1)} ms).
      </p>
    {/if}
  </div>

  {#if live}
    <div class="row column">
      <div class="row-text">
        <span class="title-medium">Now playing through</span>
        <span class="body-medium">{live.deviceName}</span>
      </div>
      <div><OutputBadge badge={live.badge} /></div>
      {#if live.fallbackReason}
        <p class="banner body-medium"><Icon name="info" size={20} /> Exclusive mode not in use: {live.fallbackReason}</p>
      {/if}
      <dl class="diag body-medium">
        <dt>Device buffer</dt>
        <dd>{live.stats.bufferFrames} frames ({ms(live.stats.bufferFrames, live.badge.output.sampleRate)})</dd>
        <dt>Period</dt>
        <dd>{live.stats.periodFrames} frames ({ms(live.stats.periodFrames, live.badge.output.sampleRate)})</dd>
        <dt>Dropouts</dt>
        <dd>{live.stats.underruns}</dd>
      </dl>
    </div>
  {/if}

  <dialog bind:this={dialog} class="dialog" aria-labelledby="loud-title">
    <Icon name="hearing" size={24} />
    <h3 id="loud-title" class="headline-small">Unlock loud levels?</h3>
    <p class="body-medium">
      In exclusive mode the DAC plays exactly what the app sends. Above −10 dB, sensitive headphones and in-ear
      monitors can get dangerously loud. Only continue if you know your setup.
    </p>
    <div class="actions">
      <Button variant="text" onclick={() => dialog?.close()}>Cancel</Button>
      <Button
        variant="filled"
        onclick={() => {
          dialog?.close();
          void store.update({}, { loudnessAcknowledged: true });
        }}>Unlock</Button
      >
    </div>
  </dialog>
{:else}
  <p class="body-medium muted">Loading audio settings…</p>
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
  .inline-row {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .select {
    min-width: 260px;
    max-width: 100%;
    height: 48px;
    padding: 0 16px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: var(--shape-sm);
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-on-surface);
  }
  .volume {
    display: grid;
    grid-template-columns: auto 1fr 5.5em;
    --slider-surface: var(--md-sys-color-surface-container-low);
    align-items: center;
    gap: 16px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .db {
    text-align: end;
    font-variant-numeric: tabular-nums;
    color: var(--md-sys-color-on-surface);
  }
  .caps-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    padding: 12px 16px;
    border-radius: var(--shape-md);
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .table-wrap {
    overflow-x: auto;
    scrollbar-gutter: auto;
  }
  .caps {
    border-collapse: separate;
    border-spacing: 3px;
    font-variant-numeric: tabular-nums;
  }
  .caps th {
    padding: 4px 8px;
    color: var(--md-sys-color-on-surface-variant);
    font-weight: 500;
    text-align: start;
    white-space: nowrap;
  }
  .caps thead th {
    text-align: center;
  }
  .caps td {
    width: 40px;
    height: 28px;
    border-radius: var(--shape-xs);
    background: var(--md-sys-color-surface-container-high);
    text-align: center;
    vertical-align: middle;
  }
  .caps td.yes {
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }
  .diag {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 24px;
    margin: 0;
  }
  .diag dt {
    color: var(--md-sys-color-on-surface-variant);
  }
  .diag dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
  .dialog {
    width: min(440px, calc(100% - 48px));
    padding: 24px;
    border: none;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    text-align: center;
  }
  .dialog::backdrop {
    background: rgb(0 0 0 / 0.32);
  }
  .dialog h3 {
    margin: 16px 0 12px;
  }
  .dialog p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    text-align: start;
  }
  .dialog .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 24px;
  }
</style>
