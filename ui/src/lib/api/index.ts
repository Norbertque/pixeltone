/**
 * The UI's only door to the Rust core. Inside the app this calls Tauri commands; in a plain
 * browser (`npm run dev`) it falls back to an in-memory mock so the UI can be developed and
 * screenshotted without Windows.
 */
import { mockBackend } from './mock';
import { tauriBackend } from './tauri';
import type { Backend } from './types';

export type * from './types';

export const inTauri: boolean = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const api: Backend = inTauri ? tauriBackend : mockBackend;
