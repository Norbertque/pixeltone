<script lang="ts">
  import { fly } from 'svelte/transition';

  import { jam } from '../jam.svelte';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';

  // Wherever the owner is in the app, above the snackbars; the Jam dialog lists them itself.
  const requests = $derived(jam.dialogOpen ? [] : (jam.view?.requests ?? []));
</script>

{#each requests as r (r.id)}
  <div class="card" transition:fly={{ y: 24, duration: 220 }}>
    <Icon name="person_add" />
    <span class="body-medium">{r.name} wants to rejoin the jam</span>
    <Button variant="text" disabled={jam.busy} onclick={() => void jam.answer(r.id, false)}>Decline</Button>
    <Button variant="filled" disabled={jam.busy} onclick={() => void jam.answer(r.id, true)}>Allow</Button>
  </div>
{/each}

<style>
  .card {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: center;
    gap: 12px;
    padding: 8px 8px 8px 16px;
    border-radius: var(--shape-lg);
    background: var(--md-sys-color-tertiary-container);
    color: var(--md-sys-color-on-tertiary-container);
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.25);
    pointer-events: auto;
  }
  .card :global(.button.text) {
    color: inherit;
  }
</style>
