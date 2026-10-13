<script lang="ts">
  import { tick } from 'svelte';

  import { dialogs } from '../dialogs.svelte';
  import Button from './Button.svelte';

  let dialog: HTMLDialogElement | undefined = $state();
  let input: HTMLInputElement | undefined = $state();
  let value = $state('');
  const req = $derived(dialogs.prompt);

  $effect(() => {
    if (!dialog) return;
    if (req && !dialog.open) {
      value = req.value;
      dialog.showModal();
      void tick().then(() => input?.select());
    } else if (!req && dialog.open) {
      dialog.close();
    }
  });

  function finish(result: string | null): void {
    const r = dialogs.prompt;
    dialogs.prompt = null;
    r?.resolve(result);
  }
</script>

<dialog bind:this={dialog} class="dialog" aria-labelledby="prompt-title" onclose={() => finish(null)}>
  {#if req}
    <form
      onsubmit={(e) => {
        e.preventDefault();
        if (value.trim()) finish(value.trim());
      }}
    >
      <h2 id="prompt-title" class="headline-small">{req.title}</h2>
      <label class="field">
        <span class="label-medium">{req.label}</span>
        <input bind:this={input} bind:value class="body-large" maxlength="200" />
      </label>
      <div class="actions">
        <Button variant="text" onclick={() => finish(null)}>Cancel</Button>
        <Button variant="filled" type="submit" disabled={!value.trim()}>{req.confirm}</Button>
      </div>
    </form>
  {/if}
</dialog>

<style>
  .dialog {
    width: min(440px, calc(100vw - 32px));
    padding: 24px;
    border: none;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
  }
  .dialog::backdrop {
    background: rgb(0 0 0 / 0.32);
  }
  h2 {
    margin: 0 0 16px;
  }
  .field {
    display: grid;
    gap: 4px;
    color: var(--md-sys-color-primary);
  }
  input {
    height: 56px;
    padding: 0 16px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: var(--shape-xs);
    background: none;
    color: var(--md-sys-color-on-surface);
  }
  input:focus {
    border-color: var(--md-sys-color-primary);
    outline: 1px solid var(--md-sys-color-primary);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 24px;
  }
</style>
