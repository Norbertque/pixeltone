<script lang="ts">
  import { fly } from 'svelte/transition';

  import { toasts } from '../toasts.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';

  const ICONS = { info: 'info', warning: 'warning', error: 'error' } as const;
</script>

<div class="stack" role="status" aria-live="polite">
  {#each toasts.items as toast (toast.id)}
    <div class={['snackbar', toast.tone]} transition:fly={{ y: 24, duration: 220 }}>
      <Icon name={ICONS[toast.tone]} size={20} />
      <p class="body-medium">{toast.message}</p>
      <IconButton icon="close" label="Dismiss" size="xs" onclick={() => toasts.dismiss(toast.id)} />
    </div>
  {/each}
</div>

<style>
  .stack {
    position: absolute;
    left: 50%;
    bottom: 120px;
    z-index: 20;
    display: grid;
    gap: 8px;
    width: min(560px, calc(100% - 32px));
    translate: -50% 0;
    pointer-events: none;
  }
  .snackbar {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 12px;
    min-height: 48px;
    padding: 6px 6px 6px 16px;
    border-radius: var(--shape-md);
    background: var(--md-sys-color-inverse-surface);
    color: var(--md-sys-color-inverse-on-surface);
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.25);
    pointer-events: auto;
  }
  .snackbar :global(.icon-button) {
    color: var(--md-sys-color-inverse-on-surface);
  }
  .warning > :global(.icon:first-child) {
    color: var(--md-sys-color-inverse-primary);
  }
  .error > :global(.icon:first-child) {
    color: var(--md-sys-color-error-container);
  }
  p {
    margin: 6px 0;
    user-select: text;
  }
</style>
