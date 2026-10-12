<script lang="ts">
  import { tick } from 'svelte';

  import { menu, type MenuItem, type OpenMenu } from '../menu.svelte';
  import Icon from './Icon.svelte';

  const MARGIN = 8;
  let el: HTMLDivElement | undefined = $state();
  const open = $derived(menu.current);

  $effect(() => {
    const m = open;
    if (!el) return;
    if (!m) {
      if (el.matches(':popover-open')) el.hidePopover();
      return;
    }
    // Show first so it can be measured, then place it inside the window.
    el.style.left = '0px';
    el.style.top = '0px';
    if (!el.matches(':popover-open')) el.showPopover();
    void tick().then(() => {
      if (!el || menu.current !== m) return;
      place(el, m.at);
      focusItem(0);
    });
  });

  function clamp(v: number, lo: number, hi: number): number {
    return Math.max(lo, Math.min(v, Math.max(lo, hi)));
  }

  function place(panel: HTMLElement, at: OpenMenu['at']): void {
    const w = panel.offsetWidth;
    const h = panel.offsetHeight;
    let x: number;
    let y: number;
    if (at instanceof HTMLElement) {
      // Under the button, right edges aligned; above it if there's no room below.
      const r = at.getBoundingClientRect();
      x = r.right - w;
      y = r.bottom + 4;
      if (y + h > innerHeight - MARGIN) y = r.top - h - 4;
      panel.style.transformOrigin = y < r.top ? 'bottom right' : 'top right';
    } else {
      // At the pointer, opening away from the window edges.
      x = at.x + w > innerWidth - MARGIN ? at.x - w : at.x;
      y = at.y + h > innerHeight - MARGIN ? at.y - h : at.y;
      panel.style.transformOrigin = `${x < at.x ? 'right' : 'left'} ${y < at.y ? 'bottom' : 'top'}`;
    }
    panel.style.left = `${clamp(x, MARGIN, innerWidth - w - MARGIN)}px`;
    panel.style.top = `${clamp(y, MARGIN, innerHeight - h - MARGIN)}px`;
  }

  function items(): HTMLElement[] {
    return el ? [...el.querySelectorAll<HTMLElement>('[role="menuitem"]:not(:disabled)')] : [];
  }

  function focusItem(index: number): void {
    const list = items();
    if (list.length) list[(index + list.length) % list.length]!.focus();
  }

  function onkeydown(e: KeyboardEvent): void {
    const list = items();
    const i = list.indexOf(document.activeElement as HTMLElement);
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        focusItem(i + 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        focusItem(i < 0 ? -1 : i - 1);
        break;
      case 'Home':
        e.preventDefault();
        focusItem(0);
        break;
      case 'End':
        e.preventDefault();
        focusItem(-1);
        break;
      case 'Tab':
        e.preventDefault();
        el?.hidePopover();
        break;
    }
  }

  function ontoggle(e: ToggleEvent): void {
    // Toggle events arrive late: another menu may already be showing in this popover.
    if (e.newState !== 'closed' || el?.matches(':popover-open')) return;
    const back = menu.current?.returnFocus;
    // Only steal focus back when it was in the menu (keyboard), not after a click elsewhere.
    const focusInMenu = !document.activeElement || document.activeElement === document.body || el?.contains(document.activeElement);
    menu.close();
    if (focusInMenu) back?.focus({ preventScroll: true });
  }

  function run(item: MenuItem): void {
    el?.hidePopover();
    void item.action();
  }
</script>

<div bind:this={el} popover="auto" class="menu" role="menu" aria-label={open?.label} tabindex="-1" {onkeydown} {ontoggle}>
  {#if open}
    {#each open.items as entry, i (i)}
      {#if entry === 'divider'}
        <div class="divider" role="separator"></div>
      {:else}
        <button
          type="button"
          role="menuitem"
          tabindex="-1"
          class="item label-large"
          disabled={entry.disabled}
          onclick={() => run(entry)}
        >
          <Icon name={entry.icon} size={20} />
          <span>{entry.label}</span>
        </button>
      {/if}
    {/each}
  {/if}
</div>

<style>
  .menu {
    position: fixed;
    inset: auto;
    min-width: 200px;
    max-width: 300px;
    margin: 0;
    padding: 6px;
    border: none;
    border-radius: var(--shape-lg);
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
    box-shadow:
      0 2px 6px rgb(0 0 0 / 0.15),
      0 8px 24px rgb(0 0 0 / 0.12);
    opacity: 1;
    scale: 1;
    transition:
      opacity var(--motion-effects-fast-duration) var(--motion-effects-fast),
      scale var(--motion-spatial-fast-duration) var(--motion-spatial-fast),
      display var(--motion-spatial-fast-duration) allow-discrete,
      overlay var(--motion-spatial-fast-duration) allow-discrete;
  }
  .menu:not(:popover-open) {
    opacity: 0;
    scale: 0.95;
  }
  @starting-style {
    .menu:popover-open {
      opacity: 0;
      scale: 0.9;
    }
  }
  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    height: 44px;
    padding: 0 16px 0 12px;
    border: none;
    border-radius: var(--shape-md);
    background: none;
    color: inherit;
    text-align: start;
    cursor: pointer;
  }
  .item :global(.icon) {
    color: var(--md-sys-color-on-surface-variant);
  }
  .item:hover,
  .item:focus-visible {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent);
    outline: none;
  }
  .item:focus-visible {
    box-shadow: inset 0 0 0 2px var(--md-sys-color-secondary);
  }
  .item:disabled {
    opacity: 0.38;
    cursor: default;
  }
  .divider {
    height: 1px;
    margin: 6px 12px;
    background: var(--md-sys-color-outline-variant);
  }
  @media (prefers-reduced-motion: reduce) {
    .menu {
      transition: none;
    }
  }
</style>
