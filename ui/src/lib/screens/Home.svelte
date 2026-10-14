<script lang="ts">
  import { api, type HomeFeed } from '../api';
  import AlbumCard from '../components/AlbumCard.svelte';
  import Artwork from '../components/Artwork.svelte';
  import Button from '../components/Button.svelte';
  import Icon from '../components/Icon.svelte';
  import Mosaic from '../components/Mosaic.svelte';
  import ScreenHeader from '../components/ScreenHeader.svelte';
  import { addMusicFolder, library } from '../library.svelte';
  import { coverOf, formatDuration, player } from '../player.svelte';
  import { router } from '../router.svelte';
  import { formatListening } from '../stats';

  let feed = $state<HomeFeed | null>(null);
  $effect(() => {
    void library.version;
    void api.libraryHome().then((f) => (feed = f));
  });

  const mixCovers = $derived([...new Set(feed?.dailyMix.map((t) => t.cover).filter((c): c is string => !!c) ?? [])].slice(0, 4));
  const monthMs = $derived(feed?.topArtists.reduce((s, a) => s + a.ms, 0) ?? 0);

  const hour = new Date().getHours();
  const greeting = hour < 5 ? 'Good night' : hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening';

  const o = $derived(library.overview);
  const hasMusic = $derived((o?.trackCount ?? 0) > 0);
  const needsFolder = $derived(o !== null && o.folders.length === 0);
</script>

<ScreenHeader
  title={greeting}
  subtitle={hasMusic && o
    ? `${o.trackCount.toLocaleString()} songs · ${o.albumCount.toLocaleString()} albums · ${o.artistCount.toLocaleString()} artists · ${formatDuration(o.durationMs)}`
    : 'Welcome to Pixeltone'}
>
  {#snippet actions()}
    {#if hasMusic}
      <Button variant="filled" size="md" icon="shuffle" onclick={() => api.playLibrary('title', null, true)}>Shuffle all</Button>
    {/if}
    <Button variant="tonal" size="md" icon="file_open" onclick={() => player.openFiles()}>Play files…</Button>
  {/snippet}
</ScreenHeader>

{#if needsFolder}
  <section class="onboarding">
    <div class="art" aria-hidden="true"><Icon name="library_music" size={56} /></div>
    <div class="text">
      <h2 class="headline-small rounded emphasized">Add your music</h2>
      <p class="body-large">
        Pick the folder where your music lives (usually your Music folder). Pixeltone reads it once, keeps up
        with changes, and plays every file bit-perfect when exclusive mode is on.
      </p>
      <div class="buttons">
        <Button variant="filled" size="md" icon="create_new_folder" onclick={addMusicFolder}>Choose music folder</Button>
        <Button variant="text" icon="tune" onclick={() => router.go('settings')}>Audio output settings</Button>
      </div>
    </div>
  </section>
{:else if !hasMusic && library.scanning}
  <section class="onboarding">
    <div class="art" aria-hidden="true"><Icon name="hourglass_top" size={56} /></div>
    <div class="text">
      <h2 class="headline-small rounded emphasized">Reading your music…</h2>
      <p class="body-large">Your albums will appear here in a moment.</p>
    </div>
  </section>
{/if}

{#if hasMusic && feed}
  <section class="made" aria-label="Made for you">
    {#if feed.dailyMix.length}
      <div class="mix">
        <button class="mix-open" onclick={() => api.playDailyMix(0)} title="Play today's mix">
          <Mosaic covers={mixCovers} size="132px" radius="var(--shape-xl)" />
          <span class="mix-text">
            <span class="overline label-large">Made for you</span>
            <span class="headline-small rounded emphasized">Today's mix</span>
            <span class="body-medium muted">
              {feed.dailyMix.length} songs from the artists you play most, favourites you haven't heard in a while, and a
              few you rarely play. New every day.
            </span>
          </span>
        </button>
        <Button variant="filled" size="md" icon="play_arrow" onclick={() => api.playDailyMix(0)}>Play</Button>
      </div>
    {/if}
    <button class="stats-link" onclick={() => router.go('stats')}>
      <span class="stats-icon"><Icon name="insights" size={32} /></span>
      <span class="mix-text">
        <span class="title-medium">Listening stats</span>
        <span class="body-medium muted">
          {monthMs > 0 ? `${formatListening(monthMs)} with your top artists this month` : 'Your top songs, artists and hours'}
        </span>
      </span>
      <Icon name="chevron_right" />
    </button>
  </section>

  {#if feed.recentAlbums.length}
    <section class="shelf">
      <div class="shelf-head">
        <h2 class="title-large rounded emphasized">Recently played</h2>
      </div>
      <div class="carousel scroller">
        {#each feed.recentAlbums as album (album.id)}
          <div class="slot"><AlbumCard {album} /></div>
        {/each}
      </div>
    </section>
  {/if}

  {#if feed.topArtists.length}
    <section class="shelf">
      <div class="shelf-head">
        <h2 class="title-large rounded emphasized">Your artists this month</h2>
      </div>
      <div class="carousel artists scroller">
        {#each feed.topArtists as a (a.artist.id)}
          <button class="artist" onclick={() => router.go({ screen: 'artist', id: a.artist.id })}>
            <Artwork src={coverOf(a.artist.cover)} size="132px" radius="50%" />
            <span class="title-small name">{a.artist.name}</span>
            <span class="body-small muted">{a.plays} play{a.plays === 1 ? '' : 's'}</span>
          </button>
        {/each}
      </div>
    </section>
  {/if}
{/if}

{#if o && o.recentlyAdded.length > 0}
  <section class="shelf">
    <div class="shelf-head">
      <h2 class="title-large rounded emphasized">Recently added</h2>
      <Button variant="text" icon="arrow_forward" onclick={() => {
        router.libraryTab = 'albums';
        router.go('library');
      }}>All albums</Button>
    </div>
    <div class="carousel scroller">
      {#each o.recentlyAdded as album (album.id)}
        <div class="slot"><AlbumCard {album} /></div>
      {/each}
    </div>
  </section>
{/if}

<style>
  .onboarding {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 32px;
    margin: 8px 0 24px;
    padding: 32px;
    border-radius: var(--shape-2xl);
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }
  .art {
    display: grid;
    place-items: center;
    width: 120px;
    height: 120px;
    border-radius: var(--shape-2xl);
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }
  .text {
    display: grid;
    flex: 1;
    gap: 8px;
    min-width: 260px;
  }
  h2 {
    margin: 0;
  }
  .text p {
    max-width: 640px;
    margin: 0;
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }
  .shelf {
    margin: 8px 0 32px;
  }
  .shelf-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  .carousel {
    display: grid;
    grid-auto-columns: 180px;
    grid-auto-flow: column;
    gap: 20px;
    padding-bottom: 12px;
    scroll-snap-type: x proximity;
    scrollbar-gutter: auto;
  }
  .slot {
    scroll-snap-align: start;
  }
  .made {
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(240px, 1fr);
    gap: 16px;
    margin: 8px 0 32px;
  }
  .mix {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
    padding: 16px 24px 16px 16px;
    border-radius: var(--shape-2xl);
    background:
      radial-gradient(120% 120% at 0% 0%, color-mix(in srgb, var(--md-sys-color-tertiary-container) 80%, transparent), transparent 70%),
      var(--md-sys-color-surface-container);
  }
  .mix-open {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 20px;
    min-width: 260px;
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    text-align: start;
    cursor: pointer;
  }
  .mix-text {
    display: grid;
    gap: 4px;
  }
  .overline {
    color: var(--md-sys-color-tertiary);
    text-transform: uppercase;
    letter-spacing: 1px;
  }
  .muted {
    color: var(--md-sys-color-on-surface-variant);
  }
  .stats-link {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 20px;
    border: none;
    border-radius: var(--shape-2xl);
    background: var(--md-sys-color-surface-container);
    color: inherit;
    text-align: start;
    cursor: pointer;
  }
  .stats-link:hover,
  .mix:hover {
    background-color: var(--md-sys-color-surface-container-high);
  }
  .stats-icon {
    display: grid;
    flex: none;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: var(--shape-lg);
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .stats-link .mix-text {
    flex: 1;
  }
  .carousel.artists {
    grid-auto-columns: 148px;
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
  @media (width < 900px) {
    .made {
      grid-template-columns: 1fr;
    }
  }
</style>
