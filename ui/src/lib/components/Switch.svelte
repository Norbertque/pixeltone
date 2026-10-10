<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    checked: boolean;
    onchange: (checked: boolean) => void;
    label: string;
    disabled?: boolean;
  }
  let { checked, onchange, label, disabled = false }: Props = $props();
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  class={['switch', { checked }]}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="thumb">
    {#if checked}<Icon name="check" size={16} />{/if}
  </span>
</button>

<style>
  .switch {
    position: relative;
    flex: none;
    width: 52px;
    height: 32px;
    padding: 0;
    border: 2px solid var(--md-sys-color-outline);
    border-radius: var(--shape-full);
    background: var(--md-sys-color-surface-container-highest);
    cursor: pointer;
    transition:
      background-color var(--motion-effects-default-duration) var(--motion-effects-default),
      border-color var(--motion-effects-default-duration) var(--motion-effects-default);
  }
  .thumb {
    position: absolute;
    top: 50%;
    left: 6px;
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--md-sys-color-outline);
    color: var(--md-sys-color-on-primary-container);
    translate: 0 -50%;
    transition:
      left var(--motion-spatial-fast-duration) var(--motion-spatial-fast),
      width var(--motion-spatial-fast-duration) var(--motion-spatial-fast),
      height var(--motion-spatial-fast-duration) var(--motion-spatial-fast),
      background-color var(--motion-effects-default-duration) var(--motion-effects-default);
  }
  .switch:active .thumb {
    width: 28px;
    height: 28px;
    left: 0;
  }
  .checked {
    border-color: var(--md-sys-color-primary);
    background: var(--md-sys-color-primary);
  }
  .checked .thumb {
    left: 22px;
    width: 24px;
    height: 24px;
    background: var(--md-sys-color-on-primary);
  }
  .checked:active .thumb {
    left: 20px;
    width: 28px;
    height: 28px;
  }
  .switch:disabled {
    cursor: default;
    opacity: 0.38;
  }
</style>
