<script lang="ts">
  import { NAV_ITEMS, router, type Section } from '../router.svelte';
  import Icon from './Icon.svelte';

  const ITEMS = [...NAV_ITEMS, { id: 'settings', icon: 'settings', label: 'Settings' }] as const satisfies readonly {
    id: Section;
    icon: string;
    label: string;
  }[];
</script>

<nav class="bar" aria-label="Main">
  {#each ITEMS as entry (entry.id)}
    {@const active = router.section === entry.id && !router.nowPlayingOpen}
    <button
      class={['item', { active }]}
      aria-current={active ? 'page' : undefined}
      onclick={() => router.go(entry.id)}
    >
      <span class="indicator"></span>
      <span class="icon"><Icon name={entry.icon} fill={active} /></span>
      <span class="label label-medium">{entry.label}</span>
    </button>
  {/each}
</nav>

<style>
  .bar {
    display: flex;
    justify-content: space-around;
    height: 80px;
    padding: 12px 8px 16px;
    background: var(--md-sys-color-surface-container);
  }
  .item {
    display: grid;
    grid-template-rows: 32px auto;
    justify-items: center;
    gap: 4px;
    min-width: 64px;
    padding: 0;
    border: none;
    background: none;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
  }
  .indicator {
    grid-area: 1 / 1;
    width: 64px;
    height: 32px;
    border-radius: var(--shape-full);
    background: var(--md-sys-color-secondary-container);
    scale: 0.4 1;
    opacity: 0;
    transition:
      scale var(--motion-spatial-fast-duration) var(--motion-spatial-fast),
      opacity var(--motion-effects-fast-duration) var(--motion-effects-fast);
  }
  .icon {
    grid-area: 1 / 1;
    display: grid;
    place-items: center;
    z-index: 1;
  }
  .active .indicator {
    opacity: 1;
    scale: 1;
  }
  .active .icon {
    color: var(--md-sys-color-on-secondary-container);
  }
  .active .label {
    color: var(--md-sys-color-on-surface);
    font-weight: 700;
  }
</style>
