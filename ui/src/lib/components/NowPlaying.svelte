<script lang="ts">
  import { describeSource, formatTime, player } from '../player.svelte';
  import { router } from '../router.svelte';
  import Artwork from './Artwork.svelte';
  import IconButton from './IconButton.svelte';
  import LyricsPanel from './LyricsPanel.svelte';
  import OutputBadge from './OutputBadge.svelte';
  import PlayButton from './PlayButton.svelte';
  import QueuePanel from './QueuePanel.svelte';
  import VolumeSlider from './VolumeSlider.svelte';
  import WavyProgress from './WavyProgress.svelte';

  const track = $derived(player.track);

  function onkeydown(e: KeyboardEvent): void {
    if (e.key !== 'Escape' || e.defaultPrevented) return;
    // Escape belongs to an open dialog, menu or text field first.
    const target = e.target instanceof HTMLElement ? e.target : null;
    if (document.querySelector('dialog:modal, :popover-open') || target?.closest('input, textarea')) return;
    e.preventDefault();
    router.setNowPlaying(false);
  }
</script>

<svelte:window {onkeydown} />

{#if track}
  <section class={['sheet', { 'with-panel': router.panel }]} aria-labelledby="np-title" style:view-transition-name="np-sheet">
    <header class="top">
      <IconButton
        icon="keyboard_arrow_down"
        label="Close Now Playing"
        data-now-playing-close
        onclick={() => router.setNowPlaying(false)}
      />
      <p class="context label-large">
        <span class="overline">Playing from</span>
        <span>{track.album ?? 'Files'}</span>
      </p>
      <div class="panels">
        <IconButton
          icon="lyrics"
          label={router.panel === 'lyrics' ? 'Hide the lyrics' : 'Show the lyrics'}
          selected={router.panel === 'lyrics'}
          onclick={() => router.togglePanel('lyrics')}
        />
        <IconButton
          icon="queue_music"
          label={router.panel === 'queue' ? 'Hide the queue' : 'Show the queue'}
          selected={router.panel === 'queue'}
          onclick={() => router.togglePanel('queue')}
        />
      </div>
    </header>

    <div class="body">
      <div class="art-wrap">
        <Artwork
          src={player.artwork}
          alt={track.album ? `Cover of ${track.album}` : ''}
          size="100%"
          radius="var(--shape-2xl)"
          transitionName="np-art"
        />
      </div>

      <div class="panel">
        <div class="titles">
          <div class="title-row">
            <h2 id="np-title" class="display-small rounded emphasized" tabindex="-1">{track.title}</h2>
            <IconButton
              icon="favorite"
              label={player.favorite ? 'Remove from favourites' : 'Add to favourites'}
              selected={player.favorite}
              onclick={() => player.toggleFavorite()}
            />
          </div>
          {#if track.artist}<p class="artist title-large">{track.artist}</p>{/if}
          <p class="format body-medium">{[track.album, describeSource(track)].filter(Boolean).join(' · ')}</p>
        </div>

        <div class="seek">
          <WavyProgress
            value={player.progress}
            getValue={() => (player.durationMs ? player.livePosition() / player.durationMs : 0)}
            playing={player.playing}
            label="Seek"
            valueText="{formatTime(player.positionMs)} of {formatTime(player.durationMs)}"
            step={5000 / Math.max(1, player.durationMs)}
            onseek={(v) => player.seek(v * player.durationMs)}
            thickness={6}
            amplitude={4.5}
            wavelength={42}
            height={44}
          />
          <div class="times label-large">
            <span>{formatTime(player.positionMs)}</span>
            <span>{formatTime(player.durationMs)}</span>
          </div>
        </div>

        <div class="transport">
          <IconButton
            icon="shuffle"
            label={player.shuffle ? 'Shuffle: on' : 'Shuffle: off'}
            selected={player.shuffle}
            onclick={() => player.toggleShuffle()}
          />
          <IconButton icon="skip_previous" label="Previous" variant="tonal" size="md" wide onclick={() => player.previous()} />
          <PlayButton playing={player.playing} onclick={() => player.togglePlay()} size={104} />
          <IconButton icon="skip_next" label="Next" variant="tonal" size="md" wide onclick={() => player.next()} />
          <IconButton
            icon={player.repeat === 'one' ? 'repeat_one' : 'repeat'}
            label="Repeat: {player.repeat}"
            selected={player.repeat !== 'off'}
            onclick={() => player.cycleRepeat()}
          />
        </div>

        <div class="volume"><VolumeSlider compact /></div>

        {#if player.badge}
          <div class="badge"><OutputBadge badge={player.badge} /></div>
        {/if}
      </div>

      {#if router.panel}
        <div class="panel-slot">
          {#if router.panel === 'queue'}
            <QueuePanel onclose={() => (router.panel = null)} />
          {:else}
            <LyricsPanel onclose={() => (router.panel = null)} />
          {/if}
        </div>
      {/if}
    </div>
  </section>
{/if}

<style>
  .sheet {
    container: sheet / size;
    position: absolute;
    inset: 0;
    z-index: 10;
    display: grid;
    grid-template-rows: auto 1fr;
    background:
      radial-gradient(120% 90% at 15% 0%, color-mix(in srgb, var(--md-sys-color-primary-container) 70%, transparent), transparent 70%),
      var(--md-sys-color-surface-container-low);
    color: var(--md-sys-color-on-surface);
  }

  .top {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    padding: 12px 16px 0;
  }
  .top > :global(:first-child) {
    justify-self: start;
  }
  .panels {
    display: flex;
    justify-self: end;
    gap: 4px;
  }
  .title-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  .title-row :global(.icon-button) {
    flex: none;
    margin-top: 4px;
  }
  .context {
    display: grid;
    margin: 0;
    text-align: center;
  }
  .overline {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 11px;
    letter-spacing: 0.8px;
    text-transform: uppercase;
  }

  .body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    align-items: center;
    gap: clamp(24px, 5cqi, 72px);
    min-height: 0;
    padding: clamp(16px, 4cqb, 48px) clamp(24px, 6cqi, 96px);
  }
  .art-wrap {
    justify-self: end;
    width: min(100%, 68cqb, 560px);
    filter: drop-shadow(0 12px 32px rgb(0 0 0 / 0.25));
  }

  .panel {
    display: grid;
    gap: 28px;
    max-width: 560px;
  }
  .titles h2 {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .titles h2:focus {
    outline: none;
  }
  .artist {
    margin: 4px 0 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .format {
    margin: 8px 0 0;
    color: var(--md-sys-color-on-surface-variant);
  }

  .times {
    display: flex;
    justify-content: space-between;
    margin-top: 2px;
    color: var(--md-sys-color-on-surface-variant);
    font-variant-numeric: tabular-nums;
  }

  .transport {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .volume {
    width: min(100%, 360px);
    /* The slider's handle gaps take the colour behind it. */
    --slider-surface: var(--md-sys-color-surface-container-low);
  }

  .badge {
    display: flex;
  }

  .panel-slot {
    display: grid;
    align-self: stretch;
    min-height: 0;
  }
  .with-panel .body {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(320px, 400px);
    padding-right: 16px;
  }
  @container sheet (width < 1180px) {
    .with-panel .art-wrap {
      display: none;
    }
    .with-panel .body {
      grid-template-columns: minmax(0, 1fr) minmax(320px, 420px);
    }
  }

  @container sheet (width < 820px) {
    .panel-slot {
      position: absolute;
      inset: 64px 8px 8px;
      z-index: 2;
      box-shadow: 0 8px 32px rgb(0 0 0 / 0.3);
      border-radius: var(--shape-xl);
    }
    .body {
      grid-template-columns: 1fr;
      align-content: start;
      justify-items: center;
      overflow: auto;
    }
    .art-wrap {
      justify-self: center;
      width: min(100%, 42cqb, 420px);
    }
    .panel {
      width: 100%;
      text-align: center;
    }
    .title-row {
      justify-content: center;
    }
    .badge {
      justify-content: center;
    }
    .volume {
      justify-self: center;
    }
  }
</style>
