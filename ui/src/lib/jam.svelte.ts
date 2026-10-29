/**
 * The jam on this PC, if one runs (crates/pp-remote): what the Jam dialog shows, and toasts as
 * guests come and go.
 */
import { api, type GuestPermissions, type JamGuest, type JamView } from './api';
import { toasts } from './toasts.svelte';

class Jam {
  view = $state.raw<JamView | null>(null);
  /** The Jam dialog is open. */
  dialogOpen = $state(false);
  busy = $state(false);
  /** Guests removed from here: their going needs no toast. */
  #removing = new Set<string>();

  readonly running = $derived(this.view !== null);

  /** The jam as the PC says it is now; tells who came and went. */
  update(next: JamView | null): void {
    const before = this.view?.guests ?? [];
    const after = next?.guests ?? [];
    if (this.view && next) {
      for (const g of after) if (!before.some((b) => b.id === g.id)) toasts.show(`${g.name} joined the jam`);
      for (const g of before) {
        if (after.some((a) => a.id === g.id)) continue;
        if (!this.#removing.delete(g.id)) toasts.show(`${g.name} left the jam`);
      }
    }
    this.view = next;
  }

  async start(): Promise<void> {
    await this.#run(async () => this.update(await api.jamStart()));
  }

  async end(): Promise<void> {
    await this.#run(() => api.jamEnd());
  }

  async setPermissions(permissions: GuestPermissions): Promise<void> {
    await this.#run(() => api.jamPermissions(permissions));
  }

  /** Asks first: the guest's tracks still to come go with them. */
  async remove(guest: JamGuest): Promise<void> {
    const upcoming = await upcomingTracks(guest.id);
    const tracks = upcoming === 1 ? 'Their upcoming track' : `Their ${upcoming} upcoming tracks`;
    const message = `Remove ${guest.name} from the jam?${upcoming ? ` ${tracks} will be removed too.` : ''}`;
    if (!(await api.confirm(message, 'Remove guest'))) return;
    this.#removing.add(guest.id);
    await this.#run(() => api.jamRemoveGuest(guest.id));
  }

  async answer(requestId: string, allow: boolean): Promise<void> {
    await this.#run(async () => {
      if (!(await api.jamAnswer(requestId, allow))) toasts.show('That request ran out, or was already answered.');
    });
  }

  async #run(action: () => Promise<unknown>): Promise<void> {
    this.busy = true;
    try {
      await action();
    } catch (e) {
      toasts.show(String(e), 'error');
    } finally {
      this.busy = false;
    }
  }
}

/** How many of a guest's tracks are still to come in the queue. */
async function upcomingTracks(guestId: string): Promise<number> {
  const snapshot = await api.playerState();
  const rows = await api.queueItems(0, snapshot.queueLength);
  const current = snapshot.queueIndex ?? -1;
  return rows.filter((r, i) => i > current && r.guestId === guestId).length;
}

export const jam = new Jam();

export async function initJam(): Promise<void> {
  await api.onJam((view) => jam.update(view));
  jam.update(await api.jamState());
}
