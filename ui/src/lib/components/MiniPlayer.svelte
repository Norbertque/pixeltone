<script lang="ts">
  import { formatTime, player } from '../player.svelte';
  import { router } from '../router.svelte';
  import Artwork from './Artwork.svelte';
  import IconButton from './IconButton.svelte';
  import OutputBadge from './OutputBadge.svelte';
  import WavyProgress from './WavyProgress.svelte';

  const track = $derived(player.track);
</script>

{#if track}
  <section class="mini-host" aria-label="Player">
  <div class="mini">
    <button class="expand" data-mini-player-expand onclick={() => router.setNowPlaying(true)} title="Open Now Playing">
      <Artwork
        src={player.artwork}
        alt=""
        size="56px"
        radius="var(--shape-lg)"
        transitionName={router.nowPlayingOpen ? undefined : 'np-art'}
      />
      <span class="meta">
        <span class="title title-medium">{track.title}</span>
        <span class="artist body-medium">{track.artist ?? track.album ?? ''}</span>
      </span>
    </button>

    <div class="controls">
      <IconButton icon="skip_previous" label="Previous" onclick={() => player.previous()} />
      <IconButton
        icon={player.playing ? 'pause' : 'play_arrow'}
        label={player.playing ? 'Pause' : 'Play'}
        variant="filled"
        size="md"
        selected={player.playing}
        onclick={() => player.togglePlay()}
      />
      <IconButton icon="skip_next" label="Next" onclick={() => player.next()} />
    </div>

    <div class="progress">
      <span class="time label-medium">{formatTime(player.positionMs)}</span>
      <WavyProgress
        value={player.progress}
        getValue={() => (player.durationMs ? player.livePosition() / player.durationMs : 0)}
        playing={player.playing}
        label="Seek"
        valueText="{formatTime(player.positionMs)} of {formatTime(player.durationMs)}"
        step={5000 / Math.max(1, player.durationMs)}
        onseek={(v) => player.seek(v * player.durationMs)}
      />
      <span class="time label-medium">{formatTime(player.durationMs)}</span>
    </div>

    {#if player.badge}
      <div class="badge"><OutputBadge badge={player.badge} compact /></div>
    {/if}

    <div class="end">
      <IconButton icon="queue_music" label="Show the queue" onclick={() => router.toggleQueue()} />
      <IconButton icon="expand_less" label="Open Now Playing" onclick={() => router.setNowPlaying(true)} />
    </div>
  </div>
  </section>
{/if}

<style>
  .mini-host {
    container: mini / inline-size;
  }
  .mini {
    display: grid;
    grid-template-columns: minmax(180px, 1.1fr) auto minmax(200px, 2fr) auto auto;
    align-items: center;
    gap: 16px;
    height: 88px;
    padding: 0 12px 0 12px;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-high);
    box-shadow: 0 2px 6px rgb(0 0 0 / 0.08);
  }

  .expand {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 4px 8px 4px 4px;
    border: none;
    border-radius: var(--shape-lg);
    background: none;
    text-align: start;
    cursor: pointer;
  }
  .expand:hover {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent);
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
    gap: 4px;
  }

  .progress {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .time {
    min-width: 3.2em;
    color: var(--md-sys-color-on-surface-variant);
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  .badge {
    min-width: 0;
  }

  .end {
    display: flex;
    align-items: center;
  }

  @container mini (width < 1000px) {
    .badge {
      display: none;
    }
  }
  @container mini (width < 760px) {
    .mini {
      grid-template-columns: 1fr auto auto;
    }
    .progress {
      display: none;
    }
  }
</style>
