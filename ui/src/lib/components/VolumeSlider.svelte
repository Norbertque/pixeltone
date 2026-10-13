<script lang="ts">
  import { output } from '../output.svelte';
  import { formatDb, nudge, volumeIcon } from '../volume';
  import IconButton from './IconButton.svelte';
  import Slider from './Slider.svelte';

  /** The volume in charge (see `volume.ts`), with mute and the mouse wheel. */
  interface Props {
    /** Narrow value column, for Now Playing. */
    compact?: boolean;
  }
  let { compact = false }: Props = $props();

  const v = $derived(output.volume);
  const db = $derived(output.draft ?? v?.db ?? 0);
  const text = $derived(v ? (v.kind === 'fixed' ? 'Line level' : v.muted ? 'Muted' : formatDb(db, v.kind)) : '');

  function onwheel(e: WheelEvent): void {
    if (!v || v.kind === 'fixed' || e.deltaY === 0) return;
    e.preventDefault();
    output.setVolume(nudge({ ...v, db }, e.deltaY < 0 ? 1 : -1));
  }
</script>

{#if v}
  <div class={['volume', { compact }]} {onwheel}>
    <IconButton
      icon={volumeIcon({ ...v, db })}
      label={v.muted ? 'Unmute' : 'Mute'}
      selected={v.muted}
      disabled={v.kind === 'fixed'}
      onclick={() => output.toggleMute()}
    />
    <Slider
      label={v.label}
      min={v.min}
      max={v.max}
      step={v.step}
      value={db}
      valueText={text}
      disabled={v.kind === 'fixed'}
      oninput={(x) => output.setVolume(x)}
      onchange={(x) => output.setVolume(x)}
    />
    <span class="db label-large">{text}</span>
  </div>
{/if}

<style>
  .volume {
    display: grid;
    grid-template-columns: auto 1fr 5.5em;
    align-items: center;
    gap: 16px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .volume.compact {
    gap: 8px;
  }
  .db {
    text-align: end;
    font-variant-numeric: tabular-nums;
    color: var(--md-sys-color-on-surface);
  }
</style>
