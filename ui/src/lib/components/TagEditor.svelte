<script lang="ts">
  import { tick } from 'svelte';

  import { api } from '../api';
  import { dialogs } from '../dialogs.svelte';
  import { changeFrom, FIELDS, hasChanges, initialForm, invalid, shared, type FormValues, type Shared } from '../tagEdit';
  import { toasts } from '../toasts.svelte';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';

  let dialog: HTMLDialogElement | undefined = $state();
  let loading = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let common = $state<Shared | null>(null);
  let form = $state<FormValues | null>(null);
  const req = $derived(dialogs.tags);
  const single = $derived((req?.paths.length ?? 0) === 1);
  const offered = $derived(FIELDS.filter((f) => single || !f.single));
  const problems = $derived(form ? offered.map((f) => invalid(f.key, form![f.key])) : []);
  const change = $derived(
    common && form
      ? changeFrom(
          common,
          form,
          offered.map((f) => f.key),
        )
      : null,
  );

  $effect(() => {
    if (!dialog) return;
    if (req && !dialog.open) {
      dialog.showModal();
      void load(req.paths);
    } else if (!req && dialog.open) {
      dialog.close();
    }
  });

  async function load(paths: string[]): Promise<void> {
    loading = true;
    error = null;
    common = null;
    form = null;
    try {
      const all = await api.tagsRead(paths);
      if (dialogs.tags?.paths !== paths) return;
      common = shared(all);
      form = initialForm(common);
      await tick();
      dialog?.querySelector<HTMLInputElement>('input')?.focus();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function save(): Promise<void> {
    const r = dialogs.tags;
    if (!r || !change || !hasChanges(change) || problems.some(Boolean)) return;
    saving = true;
    try {
      const report = await api.tagsWrite(r.paths, change);
      if (report.failed.length) {
        toasts.show(
          `Saved ${report.written} of ${r.paths.length}. ${report.failed.slice(0, 2).join('; ')}${report.failed.length > 2 ? '…' : ''}`,
          'error',
        );
      } else {
        toasts.show(report.written === 1 ? 'Tags saved' : `Tags of ${report.written} songs saved`);
      }
      dialogs.tags = null;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="dialog"
  aria-labelledby="tags-title"
  onclose={() => {
    if (dialogs.tags) dialogs.tags = null;
  }}
>
  {#if req}
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      <h2 id="tags-title" class="headline-small">Edit tags</h2>
      <p class="subtitle body-medium">{req.label}</p>

      {#if error}
        <p class="banner body-medium" role="alert"><Icon name="error" size={20} /> {error}</p>
      {/if}

      {#if loading}
        <p class="body-medium muted">Reading the files…</p>
      {:else if form && common}
        <div class="grid">
          {#each offered as f, i (f.key)}
            <label class={['field', { short: f.numeric, bad: problems[i] }]}>
              <span class="label-medium">{f.label}</span>
              <input
                class="body-large"
                bind:value={form[f.key]}
                inputmode={f.numeric ? 'numeric' : undefined}
                placeholder={common[f.key] === null ? 'Different values: leave to keep' : ''}
                aria-invalid={problems[i] ? 'true' : undefined}
                maxlength="300"
              />
              {#if problems[i]}<span class="body-small error">{problems[i]}</span>{/if}
            </label>
          {/each}
        </div>
        <p class="note body-small">
          {req.paths.length > 1 ? 'Changes apply to every selected song. ' : ''}Written into the files themselves; the
          library updates right after.
        </p>
      {/if}

      <div class="actions">
        <Button variant="text" onclick={() => (dialogs.tags = null)}>Cancel</Button>
        <Button
          variant="filled"
          type="submit"
          disabled={saving || loading || !change || !hasChanges(change) || problems.some(Boolean)}
        >
          {saving ? 'Saving…' : 'Save'}
        </Button>
      </div>
    </form>
  {/if}
</dialog>

<style>
  .dialog {
    width: min(620px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
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
    margin: 0;
  }
  .subtitle {
    margin: 4px 0 20px;
    overflow: hidden;
    color: var(--md-sys-color-on-surface-variant);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 16px 12px;
  }
  .field {
    display: grid;
    grid-column: span 6;
    gap: 4px;
    color: var(--md-sys-color-primary);
  }
  .field.short {
    grid-column: span 2;
  }
  input {
    height: 52px;
    min-width: 0;
    padding: 0 14px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: var(--shape-xs);
    background: none;
    color: var(--md-sys-color-on-surface);
  }
  input::placeholder {
    color: var(--md-sys-color-on-surface-variant);
    font-style: italic;
    opacity: 0.8;
  }
  input:focus {
    border-color: var(--md-sys-color-primary);
    outline: 1px solid var(--md-sys-color-primary);
  }
  .bad input {
    border-color: var(--md-sys-color-error);
    outline-color: var(--md-sys-color-error);
  }
  .error {
    color: var(--md-sys-color-error);
  }
  .note,
  .muted {
    margin: 16px 0 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 16px;
    padding: 12px 16px;
    border-radius: var(--shape-md);
    background: var(--md-sys-color-error-container);
    color: var(--md-sys-color-on-error-container);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 24px;
  }
</style>
