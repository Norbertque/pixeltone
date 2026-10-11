<script lang="ts">
  import { onMount } from 'svelte';

  import { api } from '../../api';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { addMusicFolder, library } from '../../library.svelte';
  import { toasts } from '../../toasts.svelte';

  const DEFAULT_DELIMITERS = [';', '/', ' feat. ', ' ft. ', ' featuring ', ' feat ', ' ft ', '(feat. ', '(ft. ', '[feat. ', '[ft. '];

  let delimiters = $state<string[]>([]);
  let draft = $state('');
  let dirty = $state(false);

  const folders = $derived(library.overview?.folders ?? []);
  const scan = $derived(library.scan);

  onMount(async () => {
    delimiters = await api.libraryDelimiters();
  });

  async function remove(id: number, path: string): Promise<void> {
    const ok = await api.confirm(`Remove “${path}” from the library? Your files stay where they are.`, 'Remove folder');
    if (!ok) return;
    try {
      await api.libraryRemoveFolder(id);
      await library.refresh();
    } catch (e) {
      toasts.show(String(e), 'error');
    }
  }

  function addDelimiter(): void {
    // Spaces are significant (" feat. "), so only empty input is rejected.
    if (draft.length === 0 || delimiters.includes(draft)) return;
    delimiters = [...delimiters, draft];
    draft = '';
    dirty = true;
  }

  function removeDelimiter(d: string): void {
    delimiters = delimiters.filter((x) => x !== d);
    dirty = true;
  }

  async function applyDelimiters(): Promise<void> {
    await api.librarySetDelimiters(delimiters);
    dirty = false;
    toasts.show('Artist separators saved. The library is being re-read.', 'info');
  }

  /** Shows leading/trailing spaces, which matter in separators. */
  function visible(d: string): string {
    return d.replaceAll(' ', '␣');
  }

  const percent = $derived(scan && scan.total > 0 ? Math.round((scan.done / scan.total) * 100) : 0);
</script>

<div class="row column">
  <div class="row-text">
    <span class="title-medium">Music folders</span>
    <span class="body-medium">PixelPlayer reads these folders and everything below them, and watches for changes.</span>
  </div>
  {#if folders.length > 0}
    <ul class="folders">
      {#each folders as f (f.id)}
        <li class="folder">
          <Icon name="folder" />
          <span class="path body-large" title={f.path}>{f.path}</span>
          <span class="count body-small">{f.trackCount.toLocaleString()} songs</span>
          <IconButton icon="delete" label="Remove {f.path}" onclick={() => remove(f.id, f.path)} />
        </li>
      {/each}
    </ul>
  {/if}
  <div class="buttons">
    <Button variant="tonal" icon="create_new_folder" onclick={addMusicFolder}>Add folder</Button>
    <Button variant="text" icon="sync" onclick={() => api.libraryRescan()} disabled={folders.length === 0 || library.scanning}
      >Rescan</Button
    >
  </div>
  {#if scan && library.scanning}
    <div class="progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent}>
      <div class="bar" style:width="{scan.phase === 'walking' ? 8 : percent}%"></div>
    </div>
    <p class="body-small muted">
      {scan.phase === 'walking'
        ? 'Looking for music…'
        : scan.phase === 'finishing'
          ? 'Finishing up…'
          : `Reading ${scan.done.toLocaleString()} of ${scan.total.toLocaleString()} new or changed files`}
    </p>
  {:else if scan?.phase === 'done'}
    <p class="body-small muted">
      Last scan: {scan.added} added, {scan.updated} updated, {scan.removed} removed{scan.errors
        ? `, ${scan.errors} unreadable`
        : ''}.
    </p>
  {/if}
</div>

<div class="row column">
  <div class="row-text">
    <span class="title-medium">Artist separators</span>
    <span class="body-medium">
      Split “A feat. B” or “A; B” into separate artists. “&amp;” and “,” are off by default so band names stay
      whole.
    </span>
  </div>
  <div class="chips">
    {#each delimiters as d (d)}
      <span class="chip label-large">
        <code>{visible(d)}</code>
        <button onclick={() => removeDelimiter(d)} aria-label="Remove separator {d}"><Icon name="close" size={16} /></button>
      </span>
    {/each}
    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault();
        addDelimiter();
      }}
    >
      <input bind:value={draft} placeholder="Add…" aria-label="New separator" class="body-medium" />
    </form>
  </div>
  <div class="buttons">
    <Button variant="filled" onclick={applyDelimiters} disabled={!dirty}>Apply and re-read library</Button>
    <Button
      variant="text"
      onclick={() => {
        delimiters = [...DEFAULT_DELIMITERS];
        dirty = true;
      }}>Reset to defaults</Button
    >
  </div>
</div>

<style>
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px 24px;
    padding: 12px 0;
  }
  .row + .row {
    border-top: 1px solid var(--md-sys-color-outline-variant);
  }
  .row.column {
    flex-direction: column;
    align-items: stretch;
  }
  .row-text {
    display: grid;
    gap: 2px;
  }
  .row-text .body-medium,
  .muted {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .folders {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .folder {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: center;
    gap: 12px;
    padding: 4px 4px 4px 12px;
    border-radius: var(--shape-md);
    background: var(--md-sys-color-surface-container);
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .count {
    color: var(--md-sys-color-on-surface-variant);
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .progress {
    height: 6px;
    overflow: hidden;
    border-radius: var(--shape-full);
    background: var(--md-sys-color-secondary-container);
  }
  .bar {
    height: 100%;
    border-radius: inherit;
    background: var(--md-sys-color-primary);
    transition: width var(--motion-effects-default-duration) var(--motion-effects-default);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 32px;
    padding: 0 4px 0 12px;
    border-radius: var(--shape-sm);
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .chip code {
    font-family: ui-monospace, 'Cascadia Mono', Consolas, monospace;
  }
  .chip button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 50%;
    background: none;
    color: inherit;
    cursor: pointer;
  }
  .add input {
    width: 96px;
    height: 32px;
    padding: 0 12px;
    border: 1px dashed var(--md-sys-color-outline);
    border-radius: var(--shape-sm);
    background: none;
    color: var(--md-sys-color-on-surface);
  }
</style>
