<script lang="ts">
  import type { OutputBadge } from '../api';
  import Icon from './Icon.svelte';

  interface Props {
    badge: OutputBadge;
    compact?: boolean;
  }
  let { badge, compact = false }: Props = $props();

  const icon = $derived(badge.bitPerfect ? 'verified' : badge.mode === 'exclusive' ? 'tune' : 'speaker');
  const tooltip = $derived(
    badge.bitPerfect
      ? 'Bit-perfect: samples reach the device unchanged'
      : badge.mode === 'exclusive'
        ? 'Exclusive mode with conversion'
        : 'Shared mode: Windows mixes and resamples the audio',
  );
</script>

<span class={['badge', { perfect: badge.bitPerfect, compact }]} title={tooltip}>
  <Icon name={icon} size={compact ? 16 : 18} fill={badge.bitPerfect} />
  <span class="text">{badge.label}</span>
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    max-width: 100%;
    padding: 0 12px 0 8px;
    border-radius: var(--shape-sm);
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-on-surface-variant);
    font: 500 12px/16px var(--font-plain);
    letter-spacing: 0.3px;
    white-space: nowrap;
  }
  .text {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .perfect {
    background: var(--md-sys-color-tertiary-container);
    color: var(--md-sys-color-on-tertiary-container);
  }
  .compact {
    height: 24px;
    padding: 0 8px 0 6px;
    gap: 4px;
    font-size: 11px;
  }
</style>
