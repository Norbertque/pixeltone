<script lang="ts">
  import { tick } from 'svelte';

  import { output } from '../output.svelte';
  import { router } from '../router.svelte';
  import { formatDb, nudge, volumeIcon, type VolumeKind } from '../volume';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import Slider from './Slider.svelte';

  const MARGIN = 8;
  const HINTS: Record<VolumeKind, string> = {
    app: 'The app’s own level. The Windows volume applies on top.',
    device: 'The device’s own volume control: the music stays bit-perfect.',
    software: 'Exclusive mode, attenuated in the app. Bit-perfect only at 0 dB.',
    fixed: 'Fixed line level: set the volume on your amplifier.',
  };

  let button: HTMLElement | undefined = $state();
  let panel: HTMLDivElement | undefined = $state();
  let open = $state(false);

  const v = $derived(output.volume);
  const db = $derived(output.draft ?? v?.db ?? 0);
  const text = $derived(v ? (v.kind === 'fixed' ? 'Line level' : v.muted ? 'Muted' : formatDb(db, v.kind)) : '');

  async function toggle(): Promise<void> {
    if (!panel || !button) return;
    if (panel.matches(':popover-open')) {
      panel.hidePopover();
      return;
    }
    // The device volume can be changed elsewhere (Windows, the keyboard): read it fresh.
    void output.refreshHardware();
    panel.style.left = '0px';
    panel.style.top = '0px';
    panel.showPopover();
    await tick();
    const r = button.getBoundingClientRect();
    const w = panel.offsetWidth;
    const h = panel.offsetHeight;
    const above = r.top - h - MARGIN >= MARGIN;
    panel.style.left = `${Math.max(MARGIN, Math.min(r.right - w, innerWidth - w - MARGIN))}px`;
    panel.style.top = `${above ? r.top - h - MARGIN : r.bottom + MARGIN}px`;
    panel.style.transformOrigin = above ? 'bottom right' : 'top right';
    panel.querySelector<HTMLInputElement>('input[type="range"]')?.focus();
  }

  function onwheel(e: WheelEvent): void {
    if (!v || v.kind === 'fixed' || e.deltaY === 0) return;
    e.preventDefault();
    output.setVolume(nudge({ ...v, db }, e.deltaY < 0 ? 1 : -1));
  }
</script>

{#if v}
  <span class="host" bind:this={button} {onwheel}>
    <IconButton
      icon={volumeIcon({ ...v, db })}
      label="Volume: {text}"
      aria-haspopup="dialog"
      aria-expanded={open}
      onclick={() => void toggle()}
    />
  </span>

  <div
    bind:this={panel}
    popover="auto"
    class="panel"
    role="dialog"
    aria-label="Volume"
    {onwheel}
    ontoggle={(e) => (open = e.newState === 'open')}
  >
    <div class="head">
      <IconButton
        icon={volumeIcon({ ...v, db })}
        label={v.muted ? 'Unmute' : 'Mute'}
        size="xs"
        selected={v.muted}
        disabled={v.kind === 'fixed'}
        onclick={() => output.toggleMute()}
      />
      <span class="title-small">{v.label}</span>
      <span class="value label-large">{text}</span>
    </div>
    {#if v.kind !== 'fixed'}
      <Slider
        label={v.label}
        min={v.min}
        max={v.max}
        step={v.step}
        value={db}
        valueText={text}
        oninput={(x) => output.setVolume(x)}
        onchange={(x) => output.setVolume(x)}
      />
    {/if}
    <p class="hint body-small">{HINTS[v.kind]}</p>
    <button
      class="settings label-large"
      onclick={() => {
        panel?.hidePopover();
        void router.go('settings');
      }}
    >
      <Icon name="tune" size={18} /> Volume settings
    </button>
  </div>
{/if}

<style>
  .host {
    display: inline-flex;
  }
  .panel {
    --slider-surface: var(--md-sys-color-surface-container);
    position: fixed;
    inset: auto;
    width: 300px;
    margin: 0;
    padding: 12px 16px 8px;
    border: none;
    border-radius: var(--shape-lg);
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
    box-shadow:
      0 2px 6px rgb(0 0 0 / 0.15),
      0 8px 24px rgb(0 0 0 / 0.12);
    opacity: 1;
    scale: 1;
    transition:
      opacity var(--motion-effects-fast-duration) var(--motion-effects-fast),
      scale var(--motion-spatial-fast-duration) var(--motion-spatial-fast),
      display var(--motion-spatial-fast-duration) allow-discrete,
      overlay var(--motion-spatial-fast-duration) allow-discrete;
  }
  .panel:not(:popover-open) {
    opacity: 0;
    scale: 0.95;
  }
  @starting-style {
    .panel:popover-open {
      opacity: 0;
      scale: 0.9;
    }
  }
  .head {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 8px;
    margin-left: -6px;
  }
  .value {
    font-variant-numeric: tabular-nums;
  }
  .hint {
    margin: 0 0 4px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .settings {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    margin-left: -8px;
    padding: 0 10px;
    border: none;
    border-radius: var(--shape-full);
    background: none;
    color: var(--md-sys-color-primary);
    cursor: pointer;
  }
  .settings:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, transparent);
  }
  @media (prefers-reduced-motion: reduce) {
    .panel {
      transition: none;
    }
  }
</style>
