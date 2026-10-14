<script lang="ts">
  import { api, type ListeningStats } from '../api';
  import AlbumCard from '../components/AlbumCard.svelte';
  import Artwork from '../components/Artwork.svelte';
  import BarChart from '../components/BarChart.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import ScreenHeader from '../components/ScreenHeader.svelte';
  import SegmentedButtons from '../components/SegmentedButtons.svelte';
  import TrackItem from '../components/TrackItem.svelte';
  import { library, playRows } from '../library.svelte';
  import { coverOf } from '../player.svelte';
  import { router } from '../router.svelte';
  import { dailyBars, formatListening, formatTick, hourBars, monthlyBars } from '../stats';

  type Period = 7 | 30 | 365 | 0;
  const PERIODS = [
    { value: 7, label: '7 days' },
    { value: 30, label: '30 days' },
    { value: 365, label: '12 months' },
    { value: 0, label: 'All time' },
  ] as const satisfies readonly { value: Period; label: string }[];

  let period = $state<Period>(30);
  let stats = $state<ListeningStats | null>(null);
  let loading = $state(false);

  $effect(() => {
    void library.version;
    const p = period;
    loading = true;
    void api.libraryStats(p === 0 ? null : p).then((s) => {
      if (p !== period) return;
      stats = s;
      loading = false;
    });
  });

  const overTime = $derived(
    !stats ? [] : period === 7 || period === 30 ? dailyBars(stats.perDay, period) : monthlyBars(stats.perDay, period === 0 ? null : 12),
  );
  const byHour = $derived(stats ? hourBars(stats.perHour) : []);
  const topTracks = $derived(stats?.topTracks.slice(0, 10) ?? []);
  const plural = (n: number, word: string) => `${n.toLocaleString()} ${word}${n === 1 ? '' : 's'}`;
</script>

<ScreenHeader title="Listening stats" subtitle="What you've been playing in Pixeltone" />

<div class="filters">
  <SegmentedButtons label="Period" options={PERIODS} value={period} onchange={(v) => (period = v)} />
</div>

{#if stats && stats.plays === 0 && !loading}
  <EmptyState
    icon="insights"
    title="Nothing counted yet"
    body="A song counts once you've heard half of it (or four minutes). Play some music and come back."
  />
{:else if stats}
  <div class={['content', { loading }]}>
    <section class="tiles" aria-label="Summary">
      <div class="tile">
        <span class="tile-label label-large">Listening time</span>
        <span class="tile-value display-small">{formatListening(stats.totalMs)}</span>
      </div>
      <div class="tile">
        <span class="tile-label label-large">Plays</span>
        <span class="tile-value display-small">{stats.plays.toLocaleString()}</span>
      </div>
      <div class="tile">
        <span class="tile-label label-large">Different songs</span>
        <span class="tile-value display-small">{stats.distinctTracks.toLocaleString()}</span>
      </div>
    </section>

    <section class="card">
      <h2 class="title-large rounded emphasized">{period === 7 || period === 30 ? 'Listening per day' : 'Listening per month'}</h2>
      <BarChart
        name={period === 7 || period === 30 ? 'Listening time per day' : 'Listening time per month'}
        data={overTime}
        format={formatListening}
        formatTick={formatTick}
        {loading}
      />
    </section>

    <section class="card">
      <h2 class="title-large rounded emphasized">When you listen</h2>
      <BarChart name="Listening time by hour of the day" data={byHour} format={formatListening} formatTick={formatTick} plotHeight={140} {loading} />
    </section>

    {#if topTracks.length}
      <section class="card">
        <h2 class="title-large rounded emphasized">Top songs</h2>
        <ol class="top-tracks">
          {#each topTracks as t, i (t.track.path)}
            <li>
              <span class="rank title-medium">{i + 1}</span>
              <div class="row">
                <TrackItem
                  track={t.track}
                  onplay={() => playRows(
                    topTracks.map((x) => x.track).filter((x) => x.id !== 0),
                    Math.max(0, topTracks.filter((x) => x.track.id !== 0).findIndex((x) => x.track.path === t.track.path)),
                  )}
                />
              </div>
              <span class="plays label-large">{plural(t.plays, 'play')}</span>
            </li>
          {/each}
        </ol>
      </section>
    {/if}

    {#if stats.topArtists.length}
      <section class="card">
        <h2 class="title-large rounded emphasized">Top artists</h2>
        <div class="artists">
          {#each stats.topArtists.slice(0, 8) as a (a.artist.id)}
            <button class="artist" onclick={() => router.go({ screen: 'artist', id: a.artist.id })}>
              <Artwork src={coverOf(a.artist.cover)} size="96px" radius="50%" />
              <span class="title-small name">{a.artist.name}</span>
              <span class="body-small muted">{plural(a.plays, 'play')} · {formatListening(a.ms)}</span>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if stats.topAlbums.length}
      <section class="card">
        <h2 class="title-large rounded emphasized">Top albums</h2>
        <div class="albums">
          {#each stats.topAlbums.slice(0, 6) as a (a.album.id)}
            <div class="album">
              <AlbumCard album={a.album} />
              <span class="body-small muted">{plural(a.plays, 'play')}</span>
            </div>
          {/each}
        </div>
      </section>
    {/if}
  </div>
{/if}

<style>
  .filters {
    display: flex;
    margin: 8px 0 16px;
  }
  .content {
    display: grid;
    gap: 16px;
    padding-bottom: 32px;
    transition: opacity var(--motion-effects-default-duration) var(--motion-effects-default);
  }
  .content.loading {
    opacity: 0.6;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 12px;
  }
  .tile {
    display: grid;
    gap: 4px;
    padding: 20px;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container);
  }
  .tile-label,
  .muted {
    color: var(--md-sys-color-on-surface-variant);
  }
  .tile-value {
    font-weight: 600;
  }
  .card {
    padding: 20px;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-low);
  }
  h2 {
    margin: 0 0 16px;
  }
  .top-tracks {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .top-tracks li {
    display: grid;
    grid-template-columns: 2em 1fr auto;
    align-items: center;
    gap: 8px;
  }
  .row {
    height: 60px;
    min-width: 0;
  }
  .rank {
    color: var(--md-sys-color-on-surface-variant);
    text-align: end;
    font-variant-numeric: tabular-nums;
  }
  .plays {
    color: var(--md-sys-color-on-surface-variant);
    font-variant-numeric: tabular-nums;
  }
  .artists {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(128px, 1fr));
    gap: 16px;
  }
  .artist {
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 8px;
    border: none;
    border-radius: var(--shape-lg);
    background: none;
    color: inherit;
    text-align: center;
    cursor: pointer;
  }
  .artist:hover {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent);
  }
  .name {
    overflow: hidden;
    max-width: 100%;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .albums {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 20px;
  }
  .album {
    display: grid;
    gap: 2px;
  }
</style>
