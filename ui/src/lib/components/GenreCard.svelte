<script lang="ts">
  import { hexFromArgb } from '@material/material-color-utilities';

  import type { GenreRow } from '../api';
  import { router } from '../router.svelte';
  import { buildScheme } from '../theme/scheme';
  import { theme } from '../theme/theme.svelte';

  interface Props {
    genre: GenreRow;
  }
  let { genre }: Props = $props();

  /** A stable hue per genre name, turned into theme-aware container colours. */
  const colors = $derived.by(() => {
    let h = 0;
    for (const c of genre.name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
    const hue = h % 360;
    const seed = hslToArgb(hue, 0.65, 0.5);
    const scheme = buildScheme(seed, theme.dark, 'vibrant');
    return { bg: hexFromArgb(scheme.primaryContainer), fg: hexFromArgb(scheme.onPrimaryContainer) };
  });

  function hslToArgb(h: number, s: number, l: number): number {
    const k = (n: number) => (n + h / 30) % 12;
    const a = s * Math.min(l, 1 - l);
    const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
    const [r, g, b] = [f(0), f(8), f(4)].map((v) => Math.round(v * 255));
    return (0xff000000 | (r! << 16) | (g! << 8) | b!) >>> 0;
  }
</script>

<button
  class="genre"
  style:background={colors.bg}
  style:color={colors.fg}
  onclick={() => router.go({ screen: 'genre', id: genre.id })}
>
  <span class="name title-large rounded emphasized">{genre.name}</span>
  <span class="count label-medium">{genre.trackCount} songs</span>
</button>

<style>
  .genre {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    height: 112px;
    padding: 16px 18px;
    border: none;
    border-radius: var(--shape-xl);
    text-align: start;
    cursor: pointer;
    overflow: hidden;
    transition: border-radius var(--motion-spatial-fast-duration) var(--motion-spatial-fast);
  }
  .genre:hover {
    border-radius: var(--shape-2xl);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .count {
    opacity: 0.8;
  }
</style>
