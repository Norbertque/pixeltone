<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  import Icon from './Icon.svelte';

  interface Props extends HTMLButtonAttributes {
    variant?: 'filled' | 'tonal' | 'outlined' | 'text' | 'elevated';
    size?: 'sm' | 'md';
    icon?: string;
    children: Snippet;
  }

  let { variant = 'filled', size = 'sm', icon, class: className = '', children, ...rest }: Props = $props();
</script>

<button type="button" class={['button', variant, size, className, { 'has-icon': icon }]} {...rest}>
  {#if icon}<Icon name={icon} size={size === 'md' ? 24 : 20} />{/if}
  <span class="label">{@render children()}</span>
</button>

<style>
  .button {
    --h: 40px;
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: var(--h);
    padding: 0 16px;
    border: none;
    border-radius: calc(var(--h) / 2);
    font: 500 14px/20px var(--font-plain);
    letter-spacing: 0.1px;
    white-space: nowrap;
    cursor: pointer;
    isolation: isolate;
    transition:
      border-radius var(--motion-spatial-fast-duration) var(--motion-spatial-fast),
      background-color var(--motion-effects-default-duration) var(--motion-effects-default),
      color var(--motion-effects-default-duration) var(--motion-effects-default);
  }
  .has-icon {
    padding-left: 12px;
  }
  .md {
    --h: 56px;
    padding: 0 24px;
    font-size: 16px;
    gap: 8px;
  }
  .md.has-icon {
    padding-left: 20px;
  }

  .button::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    background: currentColor;
    opacity: 0;
    transition: opacity var(--motion-effects-fast-duration) var(--motion-effects-fast);
  }
  .button:hover::before {
    opacity: 0.08;
  }
  .button:focus-visible::before,
  .button:active::before {
    opacity: 0.12;
  }
  .button:active {
    border-radius: var(--shape-md);
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
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    box-shadow: inset 0 0 0 1px var(--md-sys-color-outline-variant);
  }
  .text {
    background: transparent;
    color: var(--md-sys-color-primary);
    padding: 0 12px;
  }
  .elevated {
    background: var(--md-sys-color-surface-container-low);
    color: var(--md-sys-color-primary);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.3), 0 1px 3px 1px rgb(0 0 0 / 0.15);
  }

  .button:disabled {
    cursor: default;
    opacity: 0.38;
  }
</style>
