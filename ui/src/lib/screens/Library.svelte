<script lang="ts">
  import Button from '../components/Button.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import Icon from '../components/Icon.svelte';
  import ScreenHeader from '../components/ScreenHeader.svelte';
  import { router } from '../router.svelte';

  const TABS = [
    { id: 'songs', label: 'Songs', icon: 'music_note' },
    { id: 'albums', label: 'Albums', icon: 'album' },
    { id: 'artists', label: 'Artists', icon: 'artist' },
    { id: 'genres', label: 'Genres', icon: 'category' },
    { id: 'folders', label: 'Folders', icon: 'folder' },
    { id: 'playlists', label: 'Playlists', icon: 'queue_music' },
  ] as const;
  type Tab = (typeof TABS)[number]['id'];

  let tab = $state<Tab>('songs');
  const current = $derived(TABS.find((t) => t.id === tab) ?? TABS[0]);

  function onkeydown(e: KeyboardEvent): void {
    const i = TABS.findIndex((t) => t.id === tab);
    const next = e.key === 'ArrowRight' ? i + 1 : e.key === 'ArrowLeft' ? i - 1 : -2;
    if (next === -2) return;
    e.preventDefault();
    const target = TABS[(next + TABS.length) % TABS.length];
    if (target) {
      tab = target.id;
      document.getElementById(`tab-${target.id}`)?.focus();
    }
  }
</script>

<ScreenHeader title="Library" />

<div class="tabs" role="tablist" aria-label="Library sections" tabindex="-1" {onkeydown}>
  {#each TABS as t (t.id)}
    {@const selected = t.id === tab}
    <button
      id="tab-{t.id}"
      role="tab"
      aria-selected={selected}
      aria-controls="library-panel"
      tabindex={selected ? 0 : -1}
      class={['tab', { selected }]}
      onclick={() => (tab = t.id)}
    >
      <Icon name={t.icon} size={20} fill={selected} />
      <span class="label-large">{t.label}</span>
    </button>
  {/each}
</div>

<div id="library-panel" role="tabpanel" aria-labelledby="tab-{tab}">
  <EmptyState
    icon={current.icon}
    title="No {current.label.toLowerCase()} yet"
    body="Add a music folder and PixelPlayer will organise everything into songs, albums, artists, genres and folders."
  >
    {#snippet action()}
      <Button variant="tonal" icon="create_new_folder" onclick={() => router.go('settings')}>Library settings</Button>
    {/snippet}
  </EmptyState>
</div>

<style>
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 8px;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 16px 0 12px;
    border: none;
    border-radius: var(--shape-full);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    transition:
      background-color var(--motion-effects-default-duration) var(--motion-effects-default),
      border-radius var(--motion-spatial-fast-duration) var(--motion-spatial-fast);
  }
  .tab:hover {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, var(--md-sys-color-surface-container-high));
  }
  .tab.selected {
    border-radius: var(--shape-md);
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
</style>
