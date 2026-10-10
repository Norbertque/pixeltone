<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    subtitle?: string;
    actions?: Snippet;
  }
  let { title, subtitle, actions }: Props = $props();
</script>

<header class="screen-header">
  <div class="text">
    <h1 class="display-small rounded emphasized" tabindex="-1" data-screen-heading>{title}</h1>
    {#if subtitle}<p class="body-large">{subtitle}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  .screen-header {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 16px;
    padding: 36px 0 20px;
  }
  h1 {
    margin: 0;
  }
  h1:focus {
    outline: none;
  }
  p {
    margin: 6px 0 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .actions {
    display: flex;
    gap: 8px;
  }
</style>
