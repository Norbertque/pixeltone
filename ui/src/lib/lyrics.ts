/** Helpers for the lyrics panel: which synced line is current, and LRC time stamps. */
import type { LyricLine } from './api';

/** Index of the line playing at `positionMs` (the last one started), or -1 before the first. */
export function activeLine(lines: readonly LyricLine[], positionMs: number): number {
  let lo = 0;
  let hi = lines.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if ((lines[mid]!.timeMs ?? 0) <= positionMs) {
      found = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return found;
}

/** `[mm:ss.xx]`, as the core writes it. */
export function stamp(ms: number): string {
  const cs = Math.max(0, Math.floor(ms / 10));
  const mm = Math.floor(cs / 6000);
  const ss = Math.floor(cs / 100) % 60;
  return `[${String(mm).padStart(2, '0')}:${String(ss).padStart(2, '0')}.${String(cs % 100).padStart(2, '0')}]`;
}

const LEADING_STAMPS = /^(\s*\[\d+:\d{1,2}(?:[.:]\d{1,3})?\])+/;

/**
 * Puts the time `ms` at the start of the line the cursor is on (replacing any stamp there)
 * and moves the cursor to the start of the next line: press it as each line begins.
 */
export function stampLine(text: string, cursor: number, ms: number): { text: string; cursor: number } {
  const start = text.lastIndexOf('\n', Math.max(0, cursor - 1)) + 1;
  const endIndex = text.indexOf('\n', start);
  const end = endIndex < 0 ? text.length : endIndex;
  const line = text.slice(start, end).replace(LEADING_STAMPS, '').trimStart();
  const stamped = stamp(ms) + line;
  const next = text.slice(0, start) + stamped + text.slice(end);
  const after = start + stamped.length;
  return { text: next, cursor: after < next.length ? after + 1 : after };
}
