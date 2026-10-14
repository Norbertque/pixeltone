<script module lang="ts">
  import type { Lyrics } from '../api';

  /** Lyrics per track for this session: a lookup can take a few seconds. */
  const found = new Map<string, Lyrics | null>();
</script>

<script lang="ts">
  import { tick } from 'svelte';

  import { api, type LyricsRequest, type TrackMeta } from '../api';
  import { activeLine, stamp, stampLine } from '../lyrics';
  import { player } from '../player.svelte';
  import { toasts } from '../toasts.svelte';
  import Button from './Button.svelte';
  import EmptyState from './EmptyState.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    onclose?: () => void;
  }
  let { onclose }: Props = $props();

  const SOURCES: Record<Lyrics['source'], string> = {
    file: 'From an .lrc file next to the song',
    saved: 'Saved in Pixeltone',
    embedded: "From the song's tags",
    online: 'From LRCLIB',
  };
  /** Lines light up slightly early, as singers breathe in. */
  const LEAD_MS = 150;
  /** After the user scrolls, following the song pauses this long. */
  const HANDS_OFF_MS = 4000;

  let lyrics = $state<Lyrics | null>(null);
  let loading = $state(false);
  let editing = $state(false);
  let draft = $state('');
  let active = $state(-1);
  let listEl: HTMLElement | undefined = $state();
  let area: HTMLTextAreaElement | undefined = $state();
  let userScrolledAt = 0;

  const track = $derived(player.track);

  function request(t: TrackMeta): LyricsRequest {
    return { path: t.path, title: t.title, artist: t.artist, album: t.album, durationMs: t.durationMs };
  }

  // Load when the track changes.
  $effect(() => {
    const t = track;
    editing = false;
    active = -1;
    if (!t) {
      lyrics = null;
      loading = false;
      return;
    }
    if (found.has(t.path)) {
      lyrics = found.get(t.path) ?? null;
      loading = false;
      return;
    }
    lyrics = null;
    loading = true;
    api.lyricsFind(request(t)).then(
      (l) => {
        found.set(t.path, l);
        if (player.track?.path === t.path) {
          lyrics = l;
          loading = false;
        }
      },
      () => {
        if (player.track?.path === t.path) loading = false;
      },
    );
  });

  // Follow the song through synced lyrics.
  $effect(() => {
    const l = lyrics;
    if (!l?.synced || editing) return;
    let frame = 0;
    const step = (): void => {
      const i = activeLine(l.lines, player.livePosition() + LEAD_MS);
      if (i !== active) {
        active = i;
        if (performance.now() - userScrolledAt > HANDS_OFF_MS) void reveal(i);
      }
      frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  });

  // Scrolling by hand (wheel, scrollbar, keys) pauses following the song for a moment.
  $effect(() => {
    const el = listEl;
    if (!el) return;
    const touched = (): void => {
      userScrolledAt = performance.now();
    };
    const keys = (e: KeyboardEvent): void => {
      if (['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End', ' '].includes(e.key)) touched();
    };
    el.addEventListener('wheel', touched, { passive: true });
    el.addEventListener('pointerdown', touched);
    el.addEventListener('keydown', keys);
    return () => {
      el.removeEventListener('wheel', touched);
      el.removeEventListener('pointerdown', touched);
      el.removeEventListener('keydown', keys);
    };
  });

  async function reveal(i: number): Promise<void> {
    await tick();
    listEl?.querySelector<HTMLElement>(`[data-line="${Math.max(0, i)}"]`)?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  }

  function edit(): void {
    draft = lyrics?.text ?? '';
    editing = true;
    void tick().then(() => area?.focus());
  }

  function stampHere(): void {
    if (!area) return;
    const r = stampLine(draft, area.selectionStart, player.livePosition());
    draft = r.text;
    void tick().then(() => {
      area?.focus();
      area?.setSelectionRange(r.cursor, r.cursor);
    });
  }

  async function save(): Promise<void> {
    const t = track;
    if (!t) return;
    try {
      const l = await api.lyricsSave(request(t), draft);
      found.set(t.path, l);
      lyrics = l;
      editing = false;
      toasts.show(l.source === 'saved' ? 'Saved the lyrics in Pixeltone (the music folder is read-only)' : 'Saved the lyrics');
    } catch (e) {
      toasts.show(String(e), 'error');
    }
  }
</script>

<aside class="lyrics" aria-labelledby="lyrics-title">
  <header class="head">
    <h2 id="lyrics-title" class="title-large rounded emphasized">Lyrics</h2>
    {#if lyrics && !editing && !lyrics.instrumental}
      <IconButton icon="edit" label="Edit the lyrics" onclick={edit} />
    {/if}
    {#if onclose}
      <IconButton icon="close" label="Close the lyrics" onclick={onclose} />
    {/if}
  </header>

  {#if editing}
    <div class="editor">
      <textarea
        bind:this={area}
        bind:value={draft}
        class="body-large"
        spellcheck="false"
        aria-label="Lyrics (LRC: [mm:ss.xx] before a line makes it synced)"
        placeholder="Paste or type the words, one line each."
      ></textarea>
      <p class="hint body-small">
        To sync, put the cursor on the first line and press <strong>Stamp</strong> as each line begins: it writes the
        playback time ({stamp(player.positionMs)}) and moves on.
      </p>
      <div class="tools">
        <Button variant="tonal" icon="timer" onclick={stampHere}>Stamp</Button>
        <span class="spacer"></span>
        <Button variant="text" onclick={() => (editing = false)}>Cancel</Button>
        <Button variant="filled" disabled={!draft.trim()} onclick={save}>Save</Button>
      </div>
    </div>
  {:else if loading}
    <p class="status body-large"><Icon name="travel_explore" /> Looking for lyrics…</p>
  {:else if lyrics?.instrumental}
    <EmptyState icon="piano" title="Instrumental" body="This one has no words." />
  {:else if lyrics}
    <div class="body scroller" bind:this={listEl}>
      {#if lyrics.synced}
        <ol class="lines" aria-label="Synced lyrics">
          {#each lyrics.lines as line, i (i)}
            <li>
              <button
                type="button"
                data-line={i}
                class={['line', 'headline-small', 'rounded', { active: i === active, past: i < active }]}
                aria-current={i === active ? 'true' : undefined}
                onclick={() => player.seek(line.timeMs ?? 0)}
              >
                {line.text || '♪'}
              </button>
            </li>
          {/each}
        </ol>
      {:else}
        <div class="plain title-medium">
          {#each lyrics.lines as line, i (i)}
            <p>{line.text || ' '}</p>
          {/each}
        </div>
      {/if}
      <p class="source label-medium">{SOURCES[lyrics.source]}</p>
    </div>
  {:else}
    <EmptyState icon="lyrics" title="No lyrics found" body="Nothing in the song's folder or tags, or online. You can write them yourself.">
      {#snippet action()}
        <Button variant="tonal" icon="edit" onclick={edit}>Write lyrics</Button>
      {/snippet}
    </EmptyState>
  {/if}
</aside>

<style>
  .lyrics {
    display: grid;
    grid-template-rows: auto 1fr;
    min-width: 0;
    min-height: 0;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container);
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 12px 12px 8px 20px;
  }
  h2 {
    flex: 1;
    margin: 0;
  }
  .body {
    min-height: 0;
    padding: 8px 20px 24px;
  }
  .lines {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 30% 0;
    list-style: none;
  }
  .line {
    width: 100%;
    padding: 6px 8px;
    border: none;
    border-radius: var(--shape-md);
    background: none;
    color: var(--md-sys-color-on-surface-variant);
    text-align: start;
    cursor: pointer;
    opacity: 0.55;
    transition:
      opacity var(--motion-effects-default-duration) var(--motion-effects-default),
      color var(--motion-effects-default-duration) var(--motion-effects-default),
      scale var(--motion-spatial-default-duration) var(--motion-spatial-default);
    transform-origin: left center;
  }
  .line:hover {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent);
    opacity: 0.85;
  }
  .line.past {
    opacity: 0.4;
  }
  .line.active {
    color: var(--md-sys-color-primary);
    opacity: 1;
    scale: 1.04;
  }
  .plain p {
    margin: 0 0 6px;
  }
  .source {
    margin: 24px 8px 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 24px 20px;
    color: var(--md-sys-color-on-surface-variant);
  }
  .editor {
    display: grid;
    grid-template-rows: 1fr auto auto;
    gap: 8px;
    min-height: 0;
    padding: 0 16px 16px;
  }
  textarea {
    min-height: 0;
    padding: 12px;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: var(--shape-md);
    background: var(--md-sys-color-surface-container-low);
    color: var(--md-sys-color-on-surface);
    font-family: ui-monospace, 'Cascadia Mono', Consolas, monospace;
    resize: none;
  }
  .hint {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .spacer {
    flex: 1;
  }
  @media (prefers-reduced-motion: reduce) {
    .line {
      transition: none;
    }
  }
</style>
