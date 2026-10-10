import { tick } from 'svelte';

export type Screen = 'home' | 'search' | 'library' | 'settings' | 'about';

export const NAV_ITEMS = [
  { id: 'home', icon: 'home', label: 'Home' },
  { id: 'search', icon: 'search', label: 'Search' },
  { id: 'library', icon: 'library_music', label: 'Library' },
] as const satisfies readonly { id: Screen; icon: string; label: string }[];

const prefersReducedMotion = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

/**
 * Runs a DOM-changing state update inside a view transition when the platform supports it
 * and the user hasn't asked for reduced motion.
 */
export async function withViewTransition(update: () => void): Promise<void> {
  if (!('startViewTransition' in document) || prefersReducedMotion()) {
    update();
    await tick();
    return;
  }
  const transition = document.startViewTransition(async () => {
    update();
    await tick();
  });
  await transition.finished.catch(() => {});
}

/** Moves focus to the active screen's heading so keyboard and screen-reader users follow. */
function focusScreenHeading(): void {
  document.querySelector<HTMLElement>('[data-screen-heading]')?.focus({ preventScroll: true });
}

class Router {
  screen = $state<Screen>('home');
  nowPlayingOpen = $state(false);
  #history: Screen[] = [];

  async go(screen: Screen): Promise<void> {
    if (this.nowPlayingOpen) await this.setNowPlaying(false);
    if (screen === this.screen) return;
    this.#history.push(this.screen);
    if (this.#history.length > 50) this.#history.shift();
    this.screen = screen;
    await tick();
    focusScreenHeading();
  }

  async back(): Promise<void> {
    if (this.nowPlayingOpen) return this.setNowPlaying(false);
    const previous = this.#history.pop();
    if (previous) {
      this.screen = previous;
      await tick();
      focusScreenHeading();
    }
  }

  async setNowPlaying(open: boolean): Promise<void> {
    if (open === this.nowPlayingOpen) return;
    await withViewTransition(() => (this.nowPlayingOpen = open));
    const target = open ? '[data-now-playing-close]' : '[data-mini-player-expand]';
    document.querySelector<HTMLElement>(target)?.focus({ preventScroll: true });
  }
}

export const router = new Router();
