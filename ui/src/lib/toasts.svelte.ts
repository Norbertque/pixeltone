export type ToastTone = 'info' | 'warning' | 'error';

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface Toast {
  id: number;
  message: string;
  tone: ToastTone;
  action?: ToastAction;
}

class Toasts {
  items = $state<Toast[]>([]);
  #next = 1;

  /** `durationMs` 0 keeps it until dismissed. */
  show(message: string, tone: ToastTone = 'info', durationMs = 6000, action?: ToastAction): void {
    // Don't stack the same message.
    if (this.items.some((t) => t.message === message)) return;
    const id = this.#next++;
    this.items = [...this.items, { id, message, tone, action }].slice(-3);
    if (durationMs > 0) setTimeout(() => this.dismiss(id), durationMs);
  }

  dismiss(id: number): void {
    this.items = this.items.filter((t) => t.id !== id);
  }
}

export const toasts = new Toasts();
