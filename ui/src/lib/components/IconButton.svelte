<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';

  import Icon from './Icon.svelte';

  type Variant = 'standard' | 'filled' | 'tonal' | 'outlined';
  type Size = 'xs' | 'sm' | 'md' | 'lg' | 'xl';

  interface Props extends Omit<HTMLButtonAttributes, 'children'> {
    icon: string;
    /** Accessible name; also the tooltip. */
    label: string;
    variant?: Variant;
    size?: Size;
    /** Toggle buttons: pressed state (filled icon, square shape). */
    selected?: boolean;
    /** Wider than tall, like M3 Expressive's "wide" icon buttons. */
    wide?: boolean;
  }

  let {
    icon,
    label,
    variant = 'standard',
    size = 'sm',
    selected,
    wide = false,
    class: className = '',
    ...rest
  }: Props = $props();

  const DIMENSIONS: Record<Size, { box: number; icon: number }> = {
    xs: { box: 32, icon: 20 },
    sm: { box: 40, icon: 24 },
    md: { box: 56, icon: 24 },
    lg: { box: 96, icon: 32 },
    xl: { box: 136, icon: 40 },
  };
  const dims = $derived(DIMENSIONS[size]);
</script>

<button
  type="button"
  class={['icon-button', variant, size, className, { selected, wide }]}
  aria-label={label}
  title={label}
  aria-pressed={selected}
  style:--box="{dims.box}px"
  {...rest}
>
  <Icon name={icon} size={dims.icon} fill={selected ?? variant !== 'standard'} />
</button>

<style>
  .icon-button {
    --radius: calc(var(--box) / 2);
    --pressed-radius: calc(var(--box) / 4);
    position: relative;
    display: inline-grid;
    place-items: center;
    flex: none;
    width: var(--box);
    height: var(--box);
    padding: 0;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    isolation: isolate;
    transition:
      border-radius var(--motion-spatial-fast-duration) var(--motion-spatial-fast),
      width var(--motion-spatial-default-duration) var(--motion-spatial-default),
      background-color var(--motion-effects-default-duration) var(--motion-effects-default),
      color var(--motion-effects-default-duration) var(--motion-effects-default);
  }

  .wide {
    width: calc(var(--box) * 1.35);
  }

  /* State layer */
  .icon-button::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    background: currentColor;
    opacity: 0;
    transition: opacity var(--motion-effects-fast-duration) var(--motion-effects-fast);
  }
  .icon-button:hover::before {
    opacity: 0.08;
  }
  .icon-button:focus-visible::before,
  .icon-button:active::before {
    opacity: 0.12;
  }

  .icon-button:active,
  .selected {
    border-radius: var(--pressed-radius);
  }
  .selected:active {
    border-radius: var(--radius);
  }

  .filled {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }
  .tonal {
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .outlined {
    box-shadow: inset 0 0 0 1px var(--md-sys-color-outline-variant);
  }
  .standard.selected {
    color: var(--md-sys-color-primary);
  }
  .outlined.selected {
    box-shadow: none;
    background: var(--md-sys-color-inverse-surface);
    color: var(--md-sys-color-inverse-on-surface);
  }

  .icon-button:disabled {
    cursor: default;
    opacity: 0.38;
  }
  .icon-button:disabled::before {
    display: none;
  }
</style>
