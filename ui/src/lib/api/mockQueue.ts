/**
 * The play queue of the browser mock, following the engine's rules (`pp-engine` queue.rs):
 * stable ids, "play next" after the current track, shuffle that keeps the current track
 * first and remembers the original order, and the current track can't be removed.
 */
import type { QueueEntry, RepeatMode } from './types';

export interface Slot<T> {
  entry: QueueEntry;
  meta: T;
}

function shuffled<T>(items: readonly T[]): T[] {
  const a = [...items];
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j]!, a[i]!];
  }
  return a;
}

export class MockQueue<T> {
  items: Slot<T>[] = [];
  /** Original order while shuffled. */
  original: Slot<T>[] | null = null;
  current = 0;
  version = 0;
  repeat: RepeatMode = 'off';
  #lastId = 0;

  slot(entry: Omit<QueueEntry, 'id'>, meta: T): Slot<T> {
    this.#lastId += 1;
    return { entry: { ...entry, id: this.#lastId }, meta };
  }

  get playing(): Slot<T> | undefined {
    return this.items[this.current];
  }

  get shuffle(): boolean {
    return this.original !== null;
  }

  #changed(): void {
    this.version += 1;
  }

  #follow(slot: Slot<T> | undefined): void {
    this.current = slot ? Math.max(0, this.items.indexOf(slot)) : 0;
  }

  /** Replaces the queue; `shuffle` null keeps the mode. Returns the first track to play. */
  set(slots: Slot<T>[], start: number | null, shuffle: boolean | null): void {
    const on = shuffle ?? this.shuffle;
    this.original = null;
    this.items = slots;
    this.current = Math.min(start ?? 0, Math.max(0, slots.length - 1));
    if (on) this.setShuffle(true, start === null);
    this.#changed();
  }

  /** `randomLead`: no track was chosen, so any may come first. */
  setShuffle(on: boolean, randomLead = false): void {
    const playing = this.playing;
    if (on && !this.original) {
      this.original = [...this.items];
      const rest = shuffled(this.items.filter((s) => randomLead || s !== playing));
      this.items = randomLead || !playing ? rest : [playing, ...rest];
      this.current = 0;
    } else if (!on && this.original) {
      const present = new Set(this.items);
      const kept = this.original.filter((s) => present.has(s));
      const added = this.items.filter((s) => !this.original!.includes(s));
      this.items = [...kept, ...added];
      this.original = null;
      this.#follow(playing);
    }
    this.#changed();
  }

  add(slots: Slot<T>[], next: boolean): void {
    const playing = this.playing;
    if (next && playing) {
      this.items.splice(this.current + 1, 0, ...slots);
      if (this.original) {
        const at = this.original.indexOf(playing);
        this.original.splice(at + 1, 0, ...slots);
      }
    } else {
      this.items.push(...slots);
      this.original?.push(...slots);
    }
    this.#changed();
  }

  remove(ids: readonly number[]): void {
    const playing = this.playing;
    const gone = (s: Slot<T>) => s !== playing && ids.includes(s.entry.id);
    this.items = this.items.filter((s) => !gone(s));
    if (this.original) this.original = this.original.filter((s) => !gone(s));
    this.#follow(playing);
    this.#changed();
  }

  move(id: number, to: number): void {
    const playing = this.playing;
    const from = this.items.findIndex((s) => s.entry.id === id);
    if (from < 0) return;
    const [slot] = this.items.splice(from, 1);
    this.items.splice(Math.min(to, this.items.length), 0, slot!);
    this.#follow(playing);
    this.#changed();
  }

  clear(): void {
    const playing = this.playing;
    this.items = playing ? [playing] : [];
    if (this.original) this.original = [...this.items];
    this.current = 0;
    this.#changed();
  }

  jump(id: number): boolean {
    const i = this.items.findIndex((s) => s.entry.id === id);
    if (i < 0) return false;
    this.current = i;
    return true;
  }

  /** Index after the current track when it ends on its own; null at the end. */
  after(): number | null {
    const n = this.items.length;
    if (n === 0) return null;
    if (this.repeat === 'one') return this.current;
    if (this.current + 1 < n) return this.current + 1;
    return this.repeat === 'all' ? 0 : null;
  }

  /** Index for the "next" button; null at the end. */
  next(): number | null {
    const n = this.items.length;
    if (this.current + 1 < n) return this.current + 1;
    return this.repeat === 'all' && n > 0 ? 0 : null;
  }

  previous(): number {
    if (this.current > 0) return this.current - 1;
    return this.repeat === 'all' ? Math.max(0, this.items.length - 1) : 0;
  }

  entries(offset: number, limit: number): QueueEntry[] {
    return this.items.slice(offset, offset + limit).map((s) => s.entry);
  }
}
