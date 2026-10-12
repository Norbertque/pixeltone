import { api, type LibraryOverview, type ScanProgress, type TrackRow } from './api';
import { toasts } from './toasts.svelte';

class LibraryState {
  overview = $state<LibraryOverview | null>(null);
  scan = $state<ScanProgress | null>(null);
  /** Bumped whenever the library changes, so open views refetch. */
  version = $state(0);

  readonly empty = $derived((this.overview?.trackCount ?? 0) === 0);
  readonly scanning = $derived(this.scan !== null && this.scan.phase !== 'done');

  async refresh(): Promise<void> {
    try {
      this.overview = await api.libraryOverview();
    } catch (e) {
      toasts.show(`Can't read the library: ${e}`, 'error');
    }
  }
}

export const library = new LibraryState();

export async function initLibrary(): Promise<void> {
  await api.onLibraryScan((p) => {
    library.scan = p;
    if (p.phase === 'done' && p.errors > 0) {
      toasts.show(`${p.errors} file${p.errors === 1 ? '' : 's'} couldn't be read and were skipped.`, 'warning');
    }
  });
  await api.onLibraryChanged(() => {
    library.version++;
    void library.refresh();
  });
  await library.refresh();
}

/** Asks for a folder (starting in the Music folder) and adds it to the library. */
export async function addMusicFolder(): Promise<void> {
  const path = await api.pickFolder();
  if (!path) return;
  try {
    await api.libraryAddFolder(path);
    await library.refresh();
  } catch (e) {
    toasts.show(String(e), 'error');
  }
}

const ids = (rows: readonly TrackRow[]) => rows.map((r) => r.id);

/** A track was clicked: plays `rows` from it, keeping the shuffle mode. */
export function playRows(rows: readonly TrackRow[], index: number): void {
  void api.playTracks(ids(rows), index, null);
}

/** "Play" buttons: in order from the top (shuffle off). */
export function playAll(rows: readonly TrackRow[]): void {
  void api.playTracks(ids(rows), 0, false);
}

/** "Shuffle" buttons: shuffle on, starting anywhere. */
export function shuffleRows(rows: readonly TrackRow[]): void {
  void api.playTracks(ids(rows), null, true);
}

/**
 * What the confirmation says after "Play next" / "Add to queue": the song, the collection
 * (`label`, e.g. an album) or a count.
 */
export function queuedMessage(titles: readonly string[], next: boolean, label?: string): string {
  const what =
    label !== undefined ? `“${label}”` : titles.length === 1 ? `“${titles[0]}”` : `${titles.length.toLocaleString()} songs`;
  return next ? `${what} will play next` : `Added ${what} to the queue`;
}

/** "Play next" (`next`) or "Add to queue". */
export async function queueRows(rows: readonly TrackRow[], next: boolean, label?: string): Promise<void> {
  if (rows.length === 0) return;
  try {
    await api.queueAddTracks(ids(rows), next);
    toasts.show(queuedMessage(rows.map((r) => r.title), next, label));
  } catch (e) {
    toasts.show(String(e), 'error');
  }
}

/**
 * Rows fetched page by page as a virtual list scrolls. `rows` is replaced (not mutated)
 * on every page load, which keeps a 10k-row list cheap for Svelte.
 */
export class PagedRows<T> {
  rows = $state.raw<(T | undefined)[]>([]);
  #loaded = new Set<number>();
  #pending = new Set<number>();
  #reported = false;
  /** Bumped by `refresh`, so answers to older requests are dropped. */
  #generation = 0;
  #lastRange: [number, number] | null = null;
  #fetch: (offset: number, limit: number) => Promise<T[]>;
  #pageSize: number;

  constructor(count: number, fetch: (offset: number, limit: number) => Promise<T[]>, pageSize = 200) {
    this.#fetch = fetch;
    this.#pageSize = pageSize;
    this.rows = new Array<T | undefined>(count);
  }

  get count(): number {
    return this.rows.length;
  }

  /** Loads every page touching rows `start..end`. */
  ensure(start: number, end: number): void {
    this.#lastRange = [start, end];
    if (end <= start) return;
    const first = Math.floor(start / this.#pageSize);
    const last = Math.floor((end - 1) / this.#pageSize);
    for (let p = first; p <= last; p++) {
      if (!this.#loaded.has(p) && !this.#pending.has(p)) void this.#load(p);
    }
  }

  /**
   * The data changed (now `count` rows): reloads what is in view, keeping the old rows on
   * screen until the new ones arrive so nothing flashes.
   */
  refresh(count: number): void {
    this.#generation += 1;
    this.#loaded.clear();
    this.#pending.clear();
    const old = this.rows;
    this.rows = Array.from({ length: count }, (_, i) => old[i]);
    if (this.#lastRange) this.ensure(...this.#lastRange);
  }

  /** Moves a row locally (an optimistic reorder; `refresh` brings the truth). */
  move(from: number, to: number): void {
    if (from === to || from < 0 || from >= this.rows.length) return;
    const next = this.rows.slice();
    const [row] = next.splice(from, 1);
    next.splice(Math.min(to, next.length), 0, row);
    this.rows = next;
  }

  async #load(page: number): Promise<void> {
    const generation = this.#generation;
    this.#pending.add(page);
    try {
      const items = await this.#fetch(page * this.#pageSize, this.#pageSize);
      if (generation !== this.#generation) return;
      const next = this.rows.slice();
      items.forEach((item, i) => {
        const at = page * this.#pageSize + i;
        if (at < next.length) next[at] = item;
      });
      this.rows = next;
      this.#loaded.add(page);
    } catch (e) {
      // The page stays unloaded, so scrolling past it again retries. One toast per list.
      if (!this.#reported) toasts.show(`Can't read the library: ${e}`, 'error');
      this.#reported = true;
    } finally {
      if (generation === this.#generation) this.#pending.delete(page);
    }
  }
}
