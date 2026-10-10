export type ToastTone = 'info' | 'warning' | 'error';

export interface Toast {
  id: number;
  message: string;
  tone: ToastTone;
}

class Toasts {
  items = $state<Toast[]>([]);
  #next = 1;

  show(message: string, tone: ToastTone = 'info', durationMs = 6000): void {
    // Don't stack the same message.
    if (this.items.some((t) => t.message === message)) return;
    const id = this.#next++;
    this.items = [...this.items, { id, message, tone }].slice(-3);
    setTimeout(() => this.dismiss(id), durationMs);
  }

  dismiss(id: number): void {
    this.items = this.items.filter((t) => t.id !== id);
  }
}

export const toasts = new Toasts();
