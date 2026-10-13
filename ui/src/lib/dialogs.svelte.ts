/**
 * App-wide modal dialogs that any component can open: a one-field text prompt, the
 * "Add to playlist" picker and the tag editor. `PromptDialog`, `PlaylistPicker` and
 * `TagEditor` render them.
 */
import type { TrackRow } from './api';

export interface PromptRequest {
  title: string;
  label: string;
  value: string;
  confirm: string;
  resolve: (value: string | null) => void;
}

export interface PickRequest {
  trackIds: number[];
  /** What the confirmation calls them: a song or album title, or "12 songs". */
  label: string;
}

export interface TagRequest {
  paths: string[];
  /** A song title, or "12 songs". */
  label: string;
}

class Dialogs {
  prompt = $state.raw<PromptRequest | null>(null);
  pick = $state.raw<PickRequest | null>(null);
  tags = $state.raw<TagRequest | null>(null);

  /** Asks for a line of text; null when cancelled. */
  ask(title: string, opts: { label: string; confirm: string; value?: string }): Promise<string | null> {
    this.prompt?.resolve(null);
    return new Promise((resolve) => {
      this.prompt = { title, label: opts.label, confirm: opts.confirm, value: opts.value ?? '', resolve };
    });
  }

  addToPlaylist(trackIds: number[], label: string): void {
    if (trackIds.length) this.pick = { trackIds, label };
  }

  editTags(rows: readonly TrackRow[]): void {
    if (!rows.length) return;
    const label = rows.length === 1 ? rows[0]!.title : `${rows.length} songs`;
    this.tags = { paths: rows.map((r) => r.path), label };
  }
}

export const dialogs = new Dialogs();
