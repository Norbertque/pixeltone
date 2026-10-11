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

/** Plays `rows` starting at `index`. */
export function playRows(rows: readonly TrackRow[], index: number): void {
  void api.playTracks(
    rows.map((r) => r.id),
    index,
  );
}

/** Plays `rows` in random order. */
export function shuffleRows(rows: readonly TrackRow[]): void {
  const ids = rows.map((r) => r.id);
  for (let i = ids.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [ids[i], ids[j]] = [ids[j]!, ids[i]!];
  }
  void api.playTracks(ids, 0);
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
    if (end <= start) return;
    const first = Math.floor(start / this.#pageSize);
    const last = Math.floor((end - 1) / this.#pageSize);
    for (let p = first; p <= last; p++) {
      if (!this.#loaded.has(p) && !this.#pending.has(p)) void this.#load(p);
    }
  }

  async #load(page: number): Promise<void> {
    this.#pending.add(page);
    try {
      const items = await this.#fetch(page * this.#pageSize, this.#pageSize);
      const next = this.rows.slice();
      items.forEach((item, i) => (next[page * this.#pageSize + i] = item));
      this.rows = next;
      this.#loaded.add(page);
    } catch (e) {
      // The page stays unloaded, so scrolling past it again retries. One toast per list.
      if (!this.#reported) toasts.show(`Can't read the library: ${e}`, 'error');
      this.#reported = true;
    } finally {
      this.#pending.delete(page);
    }
  }
}
