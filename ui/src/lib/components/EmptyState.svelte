<script lang="ts">
  import type { Snippet } from 'svelte';

  import { scallopPath } from '../shapes';
  import Icon from './Icon.svelte';

  interface Props {
    icon: string;
    title: string;
    body?: string;
    action?: Snippet;
  }
  let { icon, title, body, action }: Props = $props();

  const shape = scallopPath(120, 10, 0.06);
</script>

<div class="empty">
  <div class="art" aria-hidden="true">
    <svg viewBox="0 0 120 120"><path d={shape} /></svg>
    <Icon name={icon} size={48} />
  </div>
  <h2 class="headline-small rounded">{title}</h2>
  {#if body}<p class="body-large">{body}</p>{/if}
  {#if action}<div class="action">{@render action()}</div>{/if}
</div>

<style>
  .empty {
    display: grid;
    justify-items: center;
    gap: 8px;
    max-width: 440px;
    margin: 48px auto;
    text-align: center;
  }
  .art {
    display: grid;
    place-items: center;
    width: 120px;
    height: 120px;
    margin-bottom: 12px;
    color: var(--md-sys-color-on-secondary-container);
  }
  .art > :global(*) {
    grid-area: 1 / 1;
  }
  svg {
    width: 100%;
    height: 100%;
    fill: var(--md-sys-color-secondary-container);
  }
  h2 {
    margin: 0;
  }
  p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .action {
    margin-top: 16px;
  }
</style>
