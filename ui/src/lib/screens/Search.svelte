<script lang="ts">
  import EmptyState from '../components/EmptyState.svelte';
  import Icon from '../components/Icon.svelte';
  import ScreenHeader from '../components/ScreenHeader.svelte';

  let query = $state('');
</script>

<ScreenHeader title="Search" />

<label class="search">
  <Icon name="search" />
  <input
    type="search"
    placeholder="Songs, albums, artists, genres"
    bind:value={query}
    class="body-large"
    aria-label="Search your library"
  />
</label>

<EmptyState
  icon="manage_search"
  title="Nothing to search yet"
  body="Search covers your whole library once your music folders are scanned."
/>

<style>
  .search {
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: 720px;
    height: 56px;
    padding: 0 20px;
    border-radius: var(--shape-full);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface-variant);
    user-select: text;
  }
  .search:focus-within {
    outline: 2px solid var(--md-sys-color-primary);
  }
  input {
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    color: var(--md-sys-color-on-surface);
    outline: none;
  }
  input::placeholder {
    color: var(--md-sys-color-on-surface-variant);
  }
</style>
