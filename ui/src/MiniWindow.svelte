<script lang="ts">
  import { api } from './lib/api';
  import Artwork from './lib/components/Artwork.svelte';
  import IconButton from './lib/components/IconButton.svelte';
  import WavyProgress from './lib/components/WavyProgress.svelte';
  import { formatTime, player } from './lib/player.svelte';
  import { theme } from './lib/theme/theme.svelte';

  const track = $derived(player.track);

  // Album colours, like the main window.
  $effect(() => {
    theme.artworkSeed = player.artworkSeed;
  });

  function onkeydown(e: KeyboardEvent): void {
    if (e.key === ' ' && !(e.target instanceof HTMLButtonElement)) {
      e.preventDefault();
      player.togglePlay();
    } else if (e.ctrlKey && e.key === 'ArrowRight') {
      player.next();
    } else if (e.ctrlKey && e.key === 'ArrowLeft') {
      player.previous();
    }
  }
</script>

<svelte:window {onkeydown} />

<main class="mini" data-tauri-drag-region>
  <div class="art" data-tauri-drag-region>
    <Artwork src={player.artwork} alt="" size="96px" radius="var(--shape-lg)" />
  </div>

  <div class="body" data-tauri-drag-region>
    <div class="top" data-tauri-drag-region>
      <div class="meta" data-tauri-drag-region>
        <span class="title title-medium" data-tauri-drag-region>{track?.title ?? 'Nothing playing'}</span>
        <span class="artist body-medium" data-tauri-drag-region>{track?.artist ?? track?.album ?? ''}</span>
      </div>
      <IconButton icon="open_in_full" label="Open the full player" size="xs" onclick={() => void api.showMainWindow()} />
    </div>

    <div class="controls" data-tauri-drag-region>
      <IconButton icon="skip_previous" label="Previous" size="xs" onclick={() => player.previous()} disabled={!track} />
      <IconButton
        icon={player.playing ? 'pause' : 'play_arrow'}
        label={player.playing ? 'Pause' : 'Play'}
        variant="filled"
        size="sm"
        selected={player.playing}
        onclick={() => player.togglePlay()}
        disabled={!track}
      />
      <IconButton icon="skip_next" label="Next" size="xs" onclick={() => player.next()} disabled={!track} />
      <span class="time label-medium" data-tauri-drag-region>
        {formatTime(player.positionMs)} / {formatTime(player.durationMs)}
      </span>
    </div>

    <WavyProgress
      value={player.progress}
      getValue={() => (player.durationMs ? player.livePosition() / player.durationMs : 0)}
      playing={player.playing}
      label="Seek"
      valueText="{formatTime(player.positionMs)} of {formatTime(player.durationMs)}"
      step={5000 / Math.max(1, player.durationMs)}
      onseek={(v) => player.seek(v * player.durationMs)}
      thickness={3}
      amplitude={2}
      wavelength={24}
      height={18}
      disabled={!track}
    />
  </div>
</main>

<style>
  :global(html),
  :global(body) {
    overflow: hidden;
  }
  .mini {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 14px;
    height: 100vh;
    padding: 12px 12px 8px 14px;
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
    user-select: none;
    -webkit-user-select: none;
  }
  .body {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .top {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: start;
    gap: 8px;
  }
  .meta {
    display: grid;
    min-width: 0;
  }
  .title,
  .artist {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .artist {
    color: var(--md-sys-color-on-surface-variant);
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .time {
    margin-left: auto;
    padding-right: 4px;
    color: var(--md-sys-color-on-surface-variant);
    font-variant-numeric: tabular-nums;
  }
</style>
