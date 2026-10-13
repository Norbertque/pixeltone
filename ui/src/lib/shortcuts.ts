/**
 * App-wide keyboard shortcuts: which key does what, and when a key belongs to the focused
 * control instead (typing in a field, Space on a button, arrows on a slider).
 */

export type ShortcutAction =
  | 'toggle'
  | 'next'
  | 'previous'
  | 'seekForward'
  | 'seekBack'
  | 'shuffle'
  | 'repeat'
  | 'volumeUp'
  | 'volumeDown'
  | 'mute'
  | 'queue'
  | 'search'
  | 'library'
  | 'settings'
  | 'openFiles'
  | 'back'
  | 'help';

export interface KeyInput {
  key: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
}

export interface KeyContext {
  /** Focus is in a text field: arrows, Space and Shift+arrows edit text there. */
  typing: boolean;
  /** The focused control acts on Space itself (buttons, switches, menu items). */
  activates: boolean;
  /** The focused control acts on arrow keys itself (sliders). */
  slider: boolean;
}

/** For the help dialog, in the order shown. */
export const SHORTCUTS: readonly { keys: readonly string[]; label: string }[] = [
  { keys: ['Space'], label: 'Play or pause' },
  { keys: ['Ctrl', '→'], label: 'Next track' },
  { keys: ['Ctrl', '←'], label: 'Previous track (or back to its start)' },
  { keys: ['Shift', '→'], label: 'Forward 10 seconds' },
  { keys: ['Shift', '←'], label: 'Back 10 seconds' },
  { keys: ['Ctrl', 'S'], label: 'Shuffle on or off' },
  { keys: ['Ctrl', 'R'], label: 'Repeat: off, all, one' },
  { keys: ['Ctrl', '↑'], label: 'Volume up' },
  { keys: ['Ctrl', '↓'], label: 'Volume down' },
  { keys: ['Ctrl', 'M'], label: 'Mute or unmute' },
  { keys: ['Ctrl', 'Q'], label: 'Show or hide the queue' },
  { keys: ['Ctrl', 'F'], label: 'Search' },
  { keys: ['Ctrl', 'L'], label: 'Library' },
  { keys: ['Ctrl', ','], label: 'Settings' },
  { keys: ['Ctrl', 'O'], label: 'Play files…' },
  { keys: ['Alt', '←'], label: 'Back' },
  { keys: ['Esc'], label: 'Close Now Playing, menus and dialogs' },
  { keys: ['?'], label: 'This list' },
];

const CTRL_KEYS: Record<string, ShortcutAction> = {
  s: 'shuffle',
  r: 'repeat',
  m: 'mute',
  q: 'queue',
  f: 'search',
  l: 'library',
  ',': 'settings',
  o: 'openFiles',
};

export function shortcutFor(e: KeyInput, ctx: KeyContext): ShortcutAction | null {
  if (e.metaKey) return null;
  const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;
  const arrowsFree = !ctx.typing && !ctx.slider;
  if (e.altKey) {
    return !e.ctrlKey && !e.shiftKey && key === 'ArrowLeft' ? 'back' : null;
  }
  if (e.ctrlKey) {
    if (e.shiftKey) return null;
    if (key === 'ArrowRight') return arrowsFree ? 'next' : null;
    if (key === 'ArrowLeft') return arrowsFree ? 'previous' : null;
    if (key === 'ArrowUp') return arrowsFree ? 'volumeUp' : null;
    if (key === 'ArrowDown') return arrowsFree ? 'volumeDown' : null;
    return CTRL_KEYS[key] ?? null;
  }
  if (ctx.typing) return null;
  if (key === '?') return 'help';
  if (e.shiftKey) {
    if (key === 'ArrowRight') return arrowsFree ? 'seekForward' : null;
    if (key === 'ArrowLeft') return arrowsFree ? 'seekBack' : null;
    return null;
  }
  if (key === ' ') return ctx.activates ? null : 'toggle';
  return null;
}
