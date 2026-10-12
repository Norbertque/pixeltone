<script lang="ts">
  import { help } from '../help.svelte';
  import { SHORTCUTS } from '../shortcuts';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';

  let dialog: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (!dialog) return;
    if (help.shortcuts && !dialog.open) dialog.showModal();
    if (!help.shortcuts && dialog.open) dialog.close();
  });
</script>

<dialog bind:this={dialog} class="dialog" aria-labelledby="shortcuts-title" onclose={() => (help.shortcuts = false)}>
  <header>
    <Icon name="keyboard" size={24} />
    <h2 id="shortcuts-title" class="headline-small">Keyboard shortcuts</h2>
  </header>
  <table class="body-medium">
    <tbody>
      {#each SHORTCUTS as s (s.label)}
        <tr>
          <td class="keys">
            {#each s.keys as k, i (k)}{#if i > 0}<span class="plus">+</span>{/if}<kbd class="label-large">{k}</kbd>{/each}
          </td>
          <td>{s.label}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  <p class="note body-medium">
    <Icon name="info" size={18} /> Your keyboard's media keys (play/pause, next, previous) work even while PixelPlayer is
    in the background.
  </p>
  <div class="actions">
    <Button variant="filled" onclick={() => (help.shortcuts = false)}>Close</Button>
  </div>
</dialog>

<style>
  .dialog {
    width: min(560px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    padding: 24px;
    border: none;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    overflow: auto;
  }
  .dialog::backdrop {
    background: rgb(0 0 0 / 0.32);
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--md-sys-color-secondary);
  }
  h2 {
    margin: 0;
    color: var(--md-sys-color-on-surface);
  }
  table {
    width: 100%;
    margin: 16px 0 8px;
    border-collapse: collapse;
  }
  td {
    padding: 6px 0;
    vertical-align: middle;
  }
  .keys {
    width: 1%;
    padding-right: 24px;
    white-space: nowrap;
  }
  kbd {
    display: inline-block;
    min-width: 28px;
    padding: 2px 8px;
    border-radius: var(--shape-sm);
    background: var(--md-sys-color-surface-container-highest);
    box-shadow: inset 0 -1px 0 var(--md-sys-color-outline-variant);
    font-family: inherit;
    text-align: center;
  }
  .plus {
    margin: 0 4px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .note {
    display: flex;
    gap: 8px;
    margin: 8px 0 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 16px;
  }
</style>
