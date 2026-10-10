<script lang="ts" generics="T extends string | number">
  import Icon from './Icon.svelte';

  interface Option {
    value: T;
    label: string;
    icon?: string;
  }
  interface Props {
    options: readonly Option[];
    value: T;
    onchange: (value: T) => void;
    label: string;
  }
  let { options, value, onchange, label }: Props = $props();
</script>

<div class="group" role="radiogroup" aria-label={label}>
  {#each options as option (option.value)}
    {@const selected = option.value === value}
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      class={['segment', { selected }]}
      onclick={() => onchange(option.value)}
    >
      {#if selected}
        <Icon name="check" size={18} />
      {:else if option.icon}
        <Icon name={option.icon} size={18} />
      {/if}
      <span class="label-large">{option.label}</span>
    </button>
  {/each}
</div>

<style>
  .group {
    display: inline-flex;
    gap: 2px;
  }
  .segment {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-width: 72px;
    height: 40px;
    padding: 0 16px;
    border: none;
    border-radius: var(--shape-sm);
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    transition:
      border-radius var(--motion-spatial-fast-duration) var(--motion-spatial-fast),
      background-color var(--motion-effects-default-duration) var(--motion-effects-default);
  }
  .segment:first-child {
    border-radius: var(--shape-full) var(--shape-sm) var(--shape-sm) var(--shape-full);
  }
  .segment:last-child {
    border-radius: var(--shape-sm) var(--shape-full) var(--shape-full) var(--shape-sm);
  }
  .segment:hover {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, var(--md-sys-color-surface-container-highest));
  }
  .segment.selected {
    border-radius: var(--shape-full);
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
</style>
