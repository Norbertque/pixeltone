import { argbFromCss, buildScheme, PALETTE_STYLES, schemeTokens, type PaletteStyle } from './scheme';

export type ThemeMode = 'system' | 'light' | 'dark';
/** Where the seed colour comes from: the playing album's artwork, or a fixed colour. */
export type ColorSource = 'artwork' | 'custom';

export const SEED_PRESETS = [
  { name: 'Lilac', hex: '#7E57C2' },
  { name: 'Ocean', hex: '#1E6FD9' },
  { name: 'Teal', hex: '#00897B' },
  { name: 'Moss', hex: '#5B8C2A' },
  { name: 'Amber', hex: '#E0A100' },
  { name: 'Coral', hex: '#F4511E' },
  { name: 'Rose', hex: '#D81B60' },
  { name: 'Slate', hex: '#546E7A' },
] as const;

const STORAGE_KEY = 'pp.theme.v1';

interface Persisted {
  mode: ThemeMode;
  style: PaletteStyle;
  source: ColorSource;
  customSeed: string;
  contrast: number;
}

function load(): Partial<Persisted> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Partial<Persisted>) : {};
  } catch {
    return {};
  }
}

class Theme {
  mode = $state<ThemeMode>('system');
  style = $state<PaletteStyle>('tonalSpot');
  source = $state<ColorSource>('artwork');
  customSeed = $state<string>(SEED_PRESETS[0].hex);
  /** −1 (reduced) … 0 (standard) … 1 (high). */
  contrast = $state(0);
  /** Seed taken from the current artwork, while something is playing. */
  artworkSeed = $state<number | null>(null);
  systemDark = $state(false);

  readonly dark = $derived(this.mode === 'system' ? this.systemDark : this.mode === 'dark');
  readonly seed = $derived(
    this.source === 'artwork' && this.artworkSeed !== null ? this.artworkSeed : argbFromCss(this.customSeed),
  );
  readonly tokens = $derived(schemeTokens(buildScheme(this.seed, this.dark, this.style, this.contrast)));

  constructor() {
    const saved = load();
    if (saved.mode === 'light' || saved.mode === 'dark' || saved.mode === 'system') this.mode = saved.mode;
    if (saved.style && (PALETTE_STYLES as readonly string[]).includes(saved.style)) this.style = saved.style;
    if (saved.source === 'artwork' || saved.source === 'custom') this.source = saved.source;
    if (typeof saved.customSeed === 'string' && /^#[0-9a-f]{6}$/i.test(saved.customSeed)) {
      this.customSeed = saved.customSeed;
    }
    if (typeof saved.contrast === 'number') this.contrast = Math.max(-1, Math.min(1, saved.contrast));
  }

  persist(): void {
    const data: Persisted = {
      mode: this.mode,
      style: this.style,
      source: this.source,
      customSeed: this.customSeed,
      contrast: this.contrast,
    };
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
    } catch {
      // Storage unavailable (private mode, blocked): settings just won't persist.
    }
  }
}

export const theme = new Theme();

let registered = false;

/** Registers every colour token as a `<color>` so theme changes can transition smoothly. */
function registerColorProperties(names: string[]): void {
  if (registered || !('registerProperty' in CSS)) return;
  registered = true;
  for (const name of names) {
    try {
      CSS.registerProperty({ name, syntax: '<color>', inherits: true, initialValue: 'transparent' });
    } catch {
      // Already registered (hot reload).
    }
  }
}

function apply(tokens: Record<string, string>, dark: boolean): void {
  const root = document.documentElement;
  const names = Object.keys(tokens);
  registerColorProperties(names);
  root.style.transitionProperty = names.join(', ');
  for (const [name, value] of Object.entries(tokens)) root.style.setProperty(name, value);
  root.style.colorScheme = dark ? 'dark' : 'light';
  root.dataset.theme = dark ? 'dark' : 'light';
}

/**
 * Starts following the system colour scheme and applies the theme whenever it changes.
 * `onApplied` receives the surface colour, e.g. to tint the native title bar.
 */
export function initTheme(onApplied?: (surface: string, dark: boolean) => void): () => void {
  const media = matchMedia('(prefers-color-scheme: dark)');
  theme.systemDark = media.matches;
  const onChange = (e: MediaQueryListEvent) => (theme.systemDark = e.matches);
  media.addEventListener('change', onChange);

  // First paint without a transition.
  apply(theme.tokens, theme.dark);

  const stop = $effect.root(() => {
    $effect(() => {
      const tokens = theme.tokens;
      const dark = theme.dark;
      apply(tokens, dark);
      onApplied?.(tokens['--md-sys-color-surface'] ?? '#000000', dark);
    });
    $effect(() => {
      // Read every persisted field so the effect re-runs when any changes.
      void [theme.mode, theme.style, theme.source, theme.customSeed, theme.contrast];
      theme.persist();
    });
  });

  return () => {
    media.removeEventListener('change', onChange);
    stop();
  };
}
