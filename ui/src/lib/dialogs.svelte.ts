/**
 * App-wide modal dialogs that any component can open: a one-field text prompt and the
 * "Add to playlist" picker. `PromptDialog` and `PlaylistPicker` render them.
 */

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

class Dialogs {
  prompt = $state.raw<PromptRequest | null>(null);
  pick = $state.raw<PickRequest | null>(null);

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
}

export const dialogs = new Dialogs();
