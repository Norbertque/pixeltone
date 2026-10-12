/**
 * The app's context menu: any component opens it at the pointer (right-click) or under a
 * "more" button; `ContextMenu.svelte` renders it once, in the top layer.
 */

export interface MenuItem {
  icon: string;
  label: string;
  action: () => unknown;
  disabled?: boolean;
}

export type MenuEntry = MenuItem | 'divider';

export interface OpenMenu {
  items: MenuEntry[];
  /** Accessible name of the menu. */
  label: string;
  /** Pointer position (right-click) or the button the menu drops from. */
  at: { x: number; y: number } | HTMLElement;
  /** Gets focus back when the menu closes from the keyboard. */
  returnFocus: HTMLElement | null;
}

class MenuState {
  current = $state.raw<OpenMenu | null>(null);

  /** Opens `items` at the pointer of a `contextmenu` event, or under a button. */
  show(from: MouseEvent | HTMLElement, items: MenuEntry[], label: string): void {
    if (from instanceof HTMLElement) {
      this.current = { items, label, at: from, returnFocus: from };
      return;
    }
    const target = from.currentTarget instanceof HTMLElement ? from.currentTarget : null;
    // The keyboard's menu key reports no pointer position: drop from the focused element.
    const at = from.clientX === 0 && from.clientY === 0 && target ? target : { x: from.clientX, y: from.clientY };
    this.current = { items, label, at, returnFocus: target };
  }

  close(): void {
    this.current = null;
  }
}

export const menu = new MenuState();

/** An `oncontextmenu` handler that opens the menu built by `items`. */
export function contextMenu(items: () => MenuEntry[], label: string): (e: MouseEvent) => void {
  return (e) => {
    e.preventDefault();
    menu.show(e, items(), label);
  };
}
