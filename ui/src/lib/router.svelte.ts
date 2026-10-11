import { tick } from 'svelte';

export type Route =
  | { screen: 'home' }
  | { screen: 'search' }
  | { screen: 'library' }
  | { screen: 'settings' }
  | { screen: 'about' }
  | { screen: 'album'; id: number }
  | { screen: 'artist'; id: number }
  | { screen: 'genre'; id: number }
  | { screen: 'folder'; path: string | null };

export type Screen = Route['screen'];
/** Destinations in the navigation rail/bar. */
export type Section = 'home' | 'search' | 'library' | 'settings';

export const NAV_ITEMS = [
  { id: 'home', icon: 'home', label: 'Home' },
  { id: 'search', icon: 'search', label: 'Search' },
  { id: 'library', icon: 'library_music', label: 'Library' },
] as const satisfies readonly { id: Section; icon: string; label: string }[];

export type LibraryTab = 'songs' | 'albums' | 'artists' | 'genres' | 'folders' | 'playlists';

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

function sectionOf(route: Route): Section {
  switch (route.screen) {
    case 'album':
    case 'artist':
    case 'genre':
    case 'folder':
      return 'library';
    case 'about':
      return 'settings';
    default:
      return route.screen;
  }
}

function sameRoute(a: Route, b: Route): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

class Router {
  route = $state<Route>({ screen: 'home' });
  nowPlayingOpen = $state(false);
  /** Remembered while browsing away and back. */
  libraryTab = $state<LibraryTab>('songs');
  readonly screen = $derived<Screen>(this.route.screen);
  readonly section = $derived<Section>(sectionOf(this.route));
  #history: Route[] = [];

  async go(target: Screen | Route): Promise<void> {
    const route = (typeof target === 'string' ? { screen: target } : target) as Route;
    if (this.nowPlayingOpen) await this.setNowPlaying(false);
    if (sameRoute(route, this.route)) return;
    this.#history.push(this.route);
    if (this.#history.length > 50) this.#history.shift();
    this.route = route;
    await tick();
    document.querySelector('main.content')?.scrollTo({ top: 0 });
    focusScreenHeading();
  }

  async back(): Promise<void> {
    if (this.nowPlayingOpen) return this.setNowPlaying(false);
    const previous = this.#history.pop();
    if (previous) {
      this.route = previous;
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
