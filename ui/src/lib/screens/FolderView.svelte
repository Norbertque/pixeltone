<script lang="ts">
  import { api, type FolderListing } from '../api';
  import Icon from '../components/Icon.svelte';
  import TrackItem from '../components/TrackItem.svelte';
  import { library, playRows } from '../library.svelte';
  import { router } from '../router.svelte';

  interface Props {
    /** `null` lists the library folders themselves. */
    path: string | null;
  }
  let { path }: Props = $props();

  let listing = $state<FolderListing | null>(null);

  $effect(() => {
    void library.version;
    const wanted = path;
    void api.libraryFolder(wanted).then((l) => {
      if (wanted === path) listing = l;
    });
  });

  /** Folders › library root › sub › folder, each clickable. */
  const crumbs = $derived.by(() => {
    if (!listing?.path) return [];
    const roots = library.overview?.folders ?? [];
    const here = listing.path;
    const root = roots.find((r) => here === r.path || here.startsWith(r.path + '\\') || here.startsWith(r.path + '/'));
    if (!root) return [{ label: listing.name, path: here }];
    const rest = here.slice(root.path.length).split(/[\\/]/).filter(Boolean);
    const out = [{ label: root.path, path: root.path }];
    let acc = root.path;
    for (const part of rest) {
      acc = `${acc}${here.includes('\\') ? '\\' : '/'}${part}`;
      out.push({ label: part, path: acc });
    }
    return out;
  });
</script>

{#if listing}
  {#if listing.path}
    <nav class="crumbs body-medium" aria-label="Folder path">
      <button onclick={() => router.go({ screen: 'library' })}>Folders</button>
      {#each crumbs as c, i (c.path)}
        <Icon name="chevron_right" size={18} />
        {#if i === crumbs.length - 1}
          <span aria-current="page">{c.label}</span>
        {:else}
          <button onclick={() => router.go({ screen: 'folder', path: c.path })}>{c.label}</button>
        {/if}
      {/each}
    </nav>
  {/if}

  <ul class="folders">
    {#each listing.folders as f (f.path)}
      <li>
        <button class="folder" onclick={() => router.go({ screen: 'folder', path: f.path })}>
          <span class="icon"><Icon name="folder" fill /></span>
          <span class="text">
            <span class="title-small name">{f.name}</span>
            <span class="body-small sub">{f.trackCount.toLocaleString()} song{f.trackCount === 1 ? '' : 's'}</span>
          </span>
          <Icon name="chevron_right" />
        </button>
      </li>
    {/each}
  </ul>

  {#if listing.tracks.length > 0}
    <div class="tracks">
      {#each listing.tracks as t, i (t.id)}
        <div class="row"><TrackItem track={t} showAlbum={false} onplay={() => playRows(listing!.tracks, i)} /></div>
      {/each}
    </div>
  {/if}
{/if}

<style>
  .crumbs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    margin: 8px 0 12px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .crumbs button {
    padding: 4px 8px;
    border: none;
    border-radius: var(--shape-sm);
    background: none;
    color: var(--md-sys-color-primary);
    cursor: pointer;
  }
  .crumbs button:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 10%, transparent);
  }
  .crumbs [aria-current] {
    padding: 4px 8px;
    color: var(--md-sys-color-on-surface);
  }
  .folders {
    display: grid;
    gap: 2px;
    margin: 8px 0;
    padding: 0;
    list-style: none;
  }
  .folder {
    display: grid;
    grid-template-columns: 40px 1fr auto;
    align-items: center;
    gap: 16px;
    width: 100%;
    height: 60px;
    padding: 0 12px 0 8px;
    border: none;
    border-radius: var(--shape-md);
    background: none;
    color: var(--md-sys-color-on-surface);
    text-align: start;
    cursor: pointer;
  }
  .folder:hover {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: var(--shape-sm);
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .text {
    display: grid;
    min-width: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    color: var(--md-sys-color-on-surface-variant);
  }
  .row {
    height: 60px;
  }
</style>
