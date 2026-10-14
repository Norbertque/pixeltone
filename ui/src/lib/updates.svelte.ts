/** New versions: a quiet check at start-up (Settings → Online), and About's button. */
import { api, type UpdateInfo } from './api';
import { toasts } from './toasts.svelte';

class Updates {
  available = $state<UpdateInfo | null>(null);
  checking = $state(false);
  /** Download progress 0…1 while installing (null: size unknown). */
  progress = $state<number | null>(null);
  installing = $state(false);
  /** The last manual check found nothing. */
  upToDate = $state(false);

  async check(manual = false): Promise<void> {
    if (this.checking) return;
    this.checking = true;
    this.upToDate = false;
    try {
      this.available = await api.checkForUpdate();
      this.upToDate = !this.available;
      if (this.available && !manual) {
        toasts.show(`Pixeltone ${this.available.version} is ready to install.`, 'info', 0, {
          label: 'Update',
          run: () => void this.install(),
        });
      }
    } catch (e) {
      // Offline at start-up is not worth a message; a manual check is.
      if (manual) toasts.show(`Can't check for updates: ${e}`, 'error');
    } finally {
      this.checking = false;
    }
  }

  async install(): Promise<void> {
    if (this.installing) return;
    this.installing = true;
    this.progress = null;
    try {
      await api.installUpdate((p) => (this.progress = p));
    } catch (e) {
      toasts.show(`The update failed: ${e}`, 'error');
    } finally {
      this.installing = false;
    }
  }
}

export const updates = new Updates();

/** Checks once, a little after start-up, unless turned off. */
export function initUpdates(): void {
  setTimeout(async () => {
    const prefs = (await api.getSettings().catch(() => null))?.preferences;
    if (prefs?.checkUpdates !== false) await updates.check();
  }, 8000);
}
