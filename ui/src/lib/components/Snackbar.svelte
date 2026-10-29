<script lang="ts">
  import { fly } from 'svelte/transition';

  import { toasts } from '../toasts.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import JamRequests from './JamRequests.svelte';

  const ICONS = { info: 'info', warning: 'warning', error: 'error' } as const;
</script>

<div class="stack" role="status" aria-live="polite">
  <JamRequests />
  {#each toasts.items as toast (toast.id)}
    <div class={['snackbar', toast.tone]} transition:fly={{ y: 24, duration: 220 }}>
      <Icon name={ICONS[toast.tone]} size={20} />
      <p class="body-medium">{toast.message}</p>
      {#if toast.action}
        {@const action = toast.action}
        <button
          class="action label-large"
          onclick={() => {
            toasts.dismiss(toast.id);
            action.run();
          }}>{action.label}</button
        >
      {/if}
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
    grid-template-columns: auto 1fr auto auto;
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
    grid-column: 2;
    margin: 6px 0;
    user-select: text;
  }
  .action {
    height: 36px;
    padding: 0 12px;
    border: none;
    border-radius: var(--shape-full);
    background: none;
    color: var(--md-sys-color-inverse-primary);
    cursor: pointer;
  }
  .action:hover {
    background: color-mix(in srgb, var(--md-sys-color-inverse-primary) 12%, transparent);
  }
</style>
