<script lang="ts">
  import { onMount } from 'svelte';

  import type { ReplayGainMode } from '../../api';
  import SegmentedButtons from '../../components/SegmentedButtons.svelte';
  import Slider from '../../components/Slider.svelte';
  import Switch from '../../components/Switch.svelte';
  import { applyPreset, BANDS, bandLabel, CUSTOM, MAX_DB, PRESETS, setBand } from '../../eq';
  import { output as store } from '../../output.svelte';
  import { formatDb } from '../../volume';

  const RG_MODES = [
    { value: 'off', label: 'Off' },
    { value: 'track', label: 'Track' },
    { value: 'album', label: 'Album' },
  ] as const satisfies readonly { value: ReplayGainMode; label: string }[];
  const MAX_PREAMP_DB = 15;
  const MAX_CROSSFADE_S = 12;

  const output = $derived(store.settings?.output ?? null);
  const eq = $derived(store.eq);
  // Drafts while dragging: these settings reopen the stream, so they're saved on release.
  let preampDraft = $state<number | null>(null);
  let crossfadeDraft = $state<number | null>(null);
  const preamp = $derived(preampDraft ?? output?.replayGainPreampDb ?? 0);
  const crossfadeS = $derived(crossfadeDraft ?? (output?.crossfadeMs ?? 0) / 1000);

  onMount(() => void store.start());

  function signed(db: number): string {
    return db > 0 ? `+${db.toFixed(1)}` : db < 0 ? `−${Math.abs(db).toFixed(1)}` : '0';
  }
</script>

{#if output && eq}
  <div class="row column">
    <div class="head">
      <div class="row-text">
        <span class="title-medium">Equaliser</span>
        <span class="body-medium">
          Ten bands, ±{MAX_DB} dB, with automatic headroom so boosts don't clip. While it's on the music is no longer
          bit-perfect, and the output badge says so.
        </span>
      </div>
      <Switch label="Equaliser" checked={eq.enabled} onchange={(v) => store.setEq({ ...eq, enabled: v })} />
    </div>

    <div class={['eq', { off: !eq.enabled }]}>
      <label class="preset">
        <span class="label-large">Preset</span>
        <select
          class="select body-large"
          value={eq.preset}
          onchange={(e) => store.setEq(applyPreset({ ...eq, enabled: true }, e.currentTarget.value))}
        >
          {#each PRESETS as p (p.name)}
            <option value={p.name}>{p.name}</option>
          {/each}
          {#if eq.preset === CUSTOM}<option value={CUSTOM}>{CUSTOM}</option>{/if}
        </select>
      </label>

      <div class="bands" role="group" aria-label="Equaliser bands">
        {#each BANDS as hz, i (hz)}
          {@const db = eq.gainsDb[i] ?? 0}
          <div class="band">
            <span class="band-db label-medium">{signed(db)}</span>
            <input
              class="vslider"
              type="range"
              min={-MAX_DB}
              max={MAX_DB}
              step="0.5"
              value={db}
              aria-label="{bandLabel(hz)}Hz"
              aria-valuetext="{signed(db)} dB"
              style:--fraction={(db + MAX_DB) / (2 * MAX_DB)}
              oninput={(e) => store.setEq(setBand({ ...eq, enabled: true }, i, Number(e.currentTarget.value)))}
              ondblclick={() => store.setEq(setBand(eq, i, 0))}
            />
            <span class="band-hz label-medium">{bandLabel(hz)}</span>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <div class="row column">
    <div class="head">
      <div class="row-text">
        <span class="title-medium">ReplayGain</span>
        <span class="body-medium">
          Evens out loudness using the ReplayGain tags in your files: per track, or per album to keep the differences
          between an album's songs. Untagged files play as they are. While it applies the music is no longer
          bit-perfect.
        </span>
      </div>
      <SegmentedButtons
        label="ReplayGain"
        options={RG_MODES}
        value={output.replayGain}
        onchange={(v) => store.update({ replayGain: v })}
      />
    </div>
    {#if output.replayGain !== 'off'}
      <div class="slider-row">
        <span class="label-large">Pre-amp</span>
        <Slider
          label="ReplayGain pre-amp"
          min={-MAX_PREAMP_DB}
          max={MAX_PREAMP_DB}
          step={0.5}
          value={preamp}
          valueText={formatDb(preamp)}
          oninput={(v) => (preampDraft = v)}
          onchange={(v) => {
            preampDraft = null;
            void store.update({ replayGainPreampDb: v });
          }}
        />
        <span class="value label-large">{formatDb(preamp)}</span>
      </div>
    {/if}
  </div>

  <div class="row column">
    <div class="head">
      <div class="row-text">
        <span class="title-medium">Crossfade</span>
        <span class="body-medium">
          Blends the end of each song into the next. Shared mode only: exclusive mode always plays gaplessly.
          {#if output.exclusive}<strong>Exclusive mode is on, so this doesn't apply now.</strong>{/if}
        </span>
      </div>
    </div>
    <div class="slider-row">
      <span class="label-large">Length</span>
      <Slider
        label="Crossfade length"
        min={0}
        max={MAX_CROSSFADE_S}
        step={0.5}
        value={crossfadeS}
        valueText={crossfadeS > 0 ? `${crossfadeS} seconds` : 'Off'}
        oninput={(v) => (crossfadeDraft = v)}
        onchange={(v) => {
          crossfadeDraft = null;
          void store.update({ crossfadeMs: Math.round(v * 1000) });
        }}
      />
      <span class="value label-large">{crossfadeS > 0 ? `${crossfadeS} s` : 'Off'}</span>
    </div>
    {#if crossfadeS > 0}
      <div class="inline-row">
        <div class="row-text">
          <span class="title-small">Crossfade within albums</span>
          <span class="body-medium">Off keeps albums gapless, as they were made, and crossfades only between them.</span>
        </div>
        <Switch
          label="Crossfade within albums"
          checked={output.crossfadeAlbums}
          onchange={(v) => store.update({ crossfadeAlbums: v })}
        />
      </div>
    {/if}
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
  .row.column {
    flex-direction: column;
    align-items: stretch;
  }
  .head,
  .inline-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px 24px;
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
  .row-text strong {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }

  .eq {
    display: grid;
    gap: 16px;
    transition: opacity var(--motion-effects-default-duration) var(--motion-effects-default);
  }
  .eq.off {
    opacity: 0.6;
  }
  .preset {
    display: flex;
    align-items: center;
    gap: 16px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .select {
    min-width: 200px;
    height: 48px;
    padding: 0 16px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: var(--shape-sm);
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-on-surface);
  }
  .bands {
    display: grid;
    grid-template-columns: repeat(10, minmax(36px, 1fr));
    gap: 4px;
    max-width: 640px;
  }
  .band {
    display: grid;
    justify-items: center;
    gap: 6px;
  }
  .band-db {
    font-variant-numeric: tabular-nums;
    color: var(--md-sys-color-on-surface);
  }
  .band-hz {
    color: var(--md-sys-color-on-surface-variant);
  }

  /* Vertical slider: a 16 px track filled from the 0 dB line toward the value. */
  .vslider {
    --track: 16px;
    --zero: 50%;
    width: 44px;
    height: 180px;
    margin: 0;
    background: transparent;
    cursor: pointer;
    appearance: none;
    writing-mode: vertical-lr;
    direction: rtl;
  }
  .vslider::-webkit-slider-runnable-track {
    width: var(--track);
    border-radius: var(--shape-full);
    background: linear-gradient(
      to top,
      var(--md-sys-color-secondary-container) 0 min(var(--zero), calc(var(--fraction) * 100%)),
      var(--md-sys-color-primary) min(var(--zero), calc(var(--fraction) * 100%))
        max(var(--zero), calc(var(--fraction) * 100%)),
      var(--md-sys-color-secondary-container) max(var(--zero), calc(var(--fraction) * 100%)) 100%
    );
  }
  .vslider::-webkit-slider-thumb {
    width: 44px;
    height: 4px;
    margin-inline-start: 0;
    margin-left: calc((var(--track) - 44px) / 2);
    border: none;
    border-radius: 2px;
    background: var(--md-sys-color-primary);
    box-shadow:
      0 -6px 0 var(--md-sys-color-surface-container-low),
      0 6px 0 var(--md-sys-color-surface-container-low);
    appearance: none;
  }
  .vslider:focus-visible {
    outline: none;
  }
  .vslider:focus-visible::-webkit-slider-thumb {
    outline: 2px solid var(--md-sys-color-secondary);
    outline-offset: 2px;
  }

  .slider-row {
    display: grid;
    grid-template-columns: 5em 1fr 5.5em;
    align-items: center;
    gap: 16px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .value {
    text-align: end;
    font-variant-numeric: tabular-nums;
    color: var(--md-sys-color-on-surface);
  }
</style>
