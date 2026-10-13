<script lang="ts">
  import { hexFromArgb } from '@material/material-color-utilities';

  import Icon from '../components/Icon.svelte';
  import ScreenHeader from '../components/ScreenHeader.svelte';
  import SegmentedButtons from '../components/SegmentedButtons.svelte';
  import { help } from '../help.svelte';
  import { router } from '../router.svelte';
  import { buildScheme, PALETTE_STYLE_LABELS, PALETTE_STYLES, type PaletteStyle } from '../theme/scheme';
  import { SEED_PRESETS, theme, type ColorSource, type ThemeMode } from '../theme/theme.svelte';
  import AudioOutput from './settings/AudioOutput.svelte';
  import LibrarySettings from './settings/LibrarySettings.svelte';
  import OnlineSettings from './settings/OnlineSettings.svelte';

  const MODES = [
    { value: 'system', label: 'System', icon: 'brightness_auto' },
    { value: 'light', label: 'Light', icon: 'light_mode' },
    { value: 'dark', label: 'Dark', icon: 'dark_mode' },
  ] as const satisfies readonly { value: ThemeMode; label: string; icon: string }[];

  const SOURCES = [
    { value: 'artwork', label: 'Album art', icon: 'album' },
    { value: 'custom', label: 'Custom', icon: 'palette' },
  ] as const satisfies readonly { value: ColorSource; label: string; icon: string }[];

  const CONTRASTS = [
    { value: 0, label: 'Standard' },
    { value: 0.5, label: 'Medium' },
    { value: 1, label: 'High' },
  ] as const;

  /** Primary, secondary and tertiary of each style for the current seed. */
  function preview(style: PaletteStyle): string[] {
    const s = buildScheme(theme.seed, theme.dark, style, theme.contrast);
    return [s.primary, s.secondaryContainer, s.tertiary].map(hexFromArgb);
  }
</script>

<ScreenHeader title="Settings" />

<div class="sections">
  <section class="section" aria-labelledby="appearance">
    <h2 id="appearance" class="title-large rounded emphasized"><Icon name="palette" /> Appearance</h2>

    <div class="row">
      <div class="row-text">
        <span class="title-medium">Theme</span>
        <span class="body-medium">Follow Windows, or always light or dark.</span>
      </div>
      <SegmentedButtons label="Theme" options={MODES} value={theme.mode} onchange={(v) => (theme.mode = v)} />
    </div>

    <div class="row">
      <div class="row-text">
        <span class="title-medium">Colour source</span>
        <span class="body-medium">Album art recolours the app for every track, like Material You.</span>
      </div>
      <SegmentedButtons label="Colour source" options={SOURCES} value={theme.source} onchange={(v) => (theme.source = v)} />
    </div>

    <div class="row">
      <div class="row-text">
        <span class="title-medium">{theme.source === 'artwork' ? 'Fallback colour' : 'Seed colour'}</span>
        <span class="body-medium">
          {theme.source === 'artwork' ? 'Used when nothing is playing or a track has no artwork.' : 'The colour every palette is built from.'}
        </span>
      </div>
      <div class="swatches" role="radiogroup" aria-label="Seed colour">
        {#each SEED_PRESETS as preset (preset.hex)}
          {@const selected = theme.customSeed.toLowerCase() === preset.hex.toLowerCase()}
          <button
            class={['swatch', { selected }]}
            role="radio"
            aria-checked={selected}
            aria-label={preset.name}
            title={preset.name}
            style:--swatch={preset.hex}
            onclick={() => (theme.customSeed = preset.hex)}
          >
            {#if selected}<Icon name="check" size={20} />{/if}
          </button>
        {/each}
        <label class="swatch custom" title="Pick any colour">
          <Icon name="colorize" size={20} />
          <input
            type="color"
            value={theme.customSeed}
            oninput={(e) => (theme.customSeed = e.currentTarget.value)}
            aria-label="Custom seed colour"
          />
        </label>
      </div>
    </div>

    <div class="row column">
      <div class="row-text">
        <span class="title-medium">Palette style</span>
        <span class="body-medium">How colourful the generated palette is.</span>
      </div>
      <div class="styles" role="radiogroup" aria-label="Palette style">
        {#each PALETTE_STYLES as style (style)}
          {@const selected = theme.style === style}
          <button class={['style', { selected }]} role="radio" aria-checked={selected} onclick={() => (theme.style = style)}>
            <span class="dots" aria-hidden="true">
              {#each preview(style) as color, i (i)}<span style:background={color}></span>{/each}
            </span>
            <span class="label-large">{PALETTE_STYLE_LABELS[style]}</span>
          </button>
        {/each}
      </div>
    </div>

    <div class="row">
      <div class="row-text">
        <span class="title-medium">Contrast</span>
      </div>
      <SegmentedButtons label="Contrast" options={CONTRASTS} value={theme.contrast} onchange={(v) => (theme.contrast = v)} />
    </div>
  </section>

  <section class="section" aria-labelledby="output">
    <h2 id="output" class="title-large rounded emphasized"><Icon name="speaker" /> Audio output</h2>
    <AudioOutput />
  </section>

  <section class="section" aria-labelledby="library">
    <h2 id="library" class="title-large rounded emphasized"><Icon name="library_music" /> Library</h2>
    <LibrarySettings />
  </section>

  <section class="section" aria-labelledby="online">
    <h2 id="online" class="title-large rounded emphasized"><Icon name="public" /> Online</h2>
    <OnlineSettings />
  </section>

  <button class="section link" onclick={() => (help.shortcuts = true)}>
    <span class="title-large rounded emphasized"><Icon name="keyboard" /> Keyboard shortcuts</span>
    <Icon name="chevron_right" />
  </button>

  <button class="section link" onclick={() => router.go('about')}>
    <span class="title-large rounded emphasized"><Icon name="info" /> About PixelPlayer PC</span>
    <Icon name="chevron_right" />
  </button>
</div>

<style>
  .sections {
    display: grid;
    gap: 16px;
    max-width: 960px;
    padding-bottom: 24px;
  }
  .section {
    display: grid;
    gap: 4px;
    padding: 20px 24px;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-low);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 0 8px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px 24px;
    padding: 12px 0;
  }
  .row + .row {
    border-top: 1px solid var(--md-sys-color-outline-variant);
  }
  .row.column {
    flex-direction: column;
    align-items: stretch;
  }
  .row-text {
    display: grid;
    gap: 2px;
  }
  .row-text .body-medium {
    color: var(--md-sys-color-on-surface-variant);
  }

  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .swatch {
    position: relative;
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    padding: 0;
    border: none;
    border-radius: var(--shape-full);
    background: var(--swatch);
    color: white;
    cursor: pointer;
    transition: border-radius var(--motion-spatial-fast-duration) var(--motion-spatial-fast);
  }
  .swatch.selected {
    border-radius: var(--shape-md);
    box-shadow:
      0 0 0 3px var(--md-sys-color-surface-container-low),
      0 0 0 5px var(--md-sys-color-on-surface);
  }
  .swatch.custom {
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-on-surface-variant);
    overflow: hidden;
  }
  .swatch.custom input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }

  .styles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 8px;
  }
  .style {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 48px;
    padding: 0 14px;
    border: none;
    border-radius: var(--shape-lg);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    text-align: start;
    cursor: pointer;
    transition: border-radius var(--motion-spatial-fast-duration) var(--motion-spatial-fast);
  }
  .style.selected {
    border-radius: var(--shape-full);
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .dots {
    display: flex;
  }
  .dots span {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px var(--md-sys-color-surface-container-high);
  }
  .dots span + span {
    margin-left: -5px;
  }

  .link {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border: none;
    color: inherit;
    cursor: pointer;
  }
  .link span {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .link:hover {
    background: var(--md-sys-color-surface-container);
  }
</style>
