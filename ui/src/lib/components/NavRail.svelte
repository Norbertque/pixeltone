<script lang="ts">
  import { jam } from '../jam.svelte';
  import { NAV_ITEMS, router, type Section } from '../router.svelte';
  import Icon from './Icon.svelte';
  import Logo from './Logo.svelte';

  const BOTTOM_ITEMS = [{ id: 'settings', icon: 'settings', label: 'Settings' }] as const satisfies readonly {
    id: Section;
    icon: string;
    label: string;
  }[];
</script>

{#snippet item(entry: { id: Section; icon: string; label: string })}
  {@const active = router.section === entry.id && !router.nowPlayingOpen}
  <li>
    <button
      class={['item', { active }]}
      aria-current={active ? 'page' : undefined}
      onclick={() => router.go(entry.id)}
    >
      <span class="indicator"></span>
      <span class="icon"><Icon name={entry.icon} fill={active} /></span>
      <span class="label label-medium">{entry.label}</span>
    </button>
  </li>
{/snippet}

<nav class="rail" aria-label="Main">
  <div class="brand">
    <Logo size={48} />
  </div>
  <ul>
    {#each NAV_ITEMS as entry (entry.id)}
      {@render item(entry)}
    {/each}
  </ul>
  <ul class="bottom">
    <li>
      <!-- Not a screen: the Jam dialog, here so a jam can start before anything is queued. -->
      <button
        class={['item', { live: jam.running }]}
        aria-label={jam.running ? 'Jam (running)' : 'Jam'}
        onclick={() => (jam.dialogOpen = true)}
      >
        <span class="indicator"></span>
        <span class="icon"><Icon name="groups" fill={jam.running} /></span>
        <span class="label label-medium">Jam</span>
      </button>
    </li>
    {#each BOTTOM_ITEMS as entry (entry.id)}
      {@render item(entry)}
    {/each}
  </ul>
</nav>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 96px;
    height: 100%;
    padding: 20px 0 16px;
  }
  .brand {
    display: grid;
    place-items: center;
    height: 56px;
    margin-bottom: 28px;
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .bottom {
    margin-top: auto;
  }

  .item {
    position: relative;
    display: grid;
    grid-template-rows: 32px auto;
    justify-items: center;
    gap: 4px;
    width: 80px;
    padding: 0;
    border: none;
    background: none;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
  }
  .indicator {
    grid-area: 1 / 1;
    width: 56px;
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
  .item:hover .indicator {
    opacity: 0.5;
    scale: 1;
  }
  .active {
    color: var(--md-sys-color-on-surface);
  }
  .active .indicator {
    opacity: 1;
    scale: 1;
  }
  .active .icon {
    color: var(--md-sys-color-on-secondary-container);
  }
  .active .label {
    font-weight: 700;
  }
  .live .icon {
    position: relative;
    color: var(--md-sys-color-primary);
  }
  .live .icon::after {
    content: '';
    position: absolute;
    top: 2px;
    right: -4px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--md-sys-color-error);
  }
  .item:focus-visible {
    outline: none;
  }
  .item:focus-visible .indicator {
    opacity: 1;
    scale: 1;
    outline: 2px solid var(--md-sys-color-secondary);
    outline-offset: 2px;
  }
</style>
