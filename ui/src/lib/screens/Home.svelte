<script lang="ts">
  import { api } from '../api';
  import AlbumCard from '../components/AlbumCard.svelte';
  import Button from '../components/Button.svelte';
  import Icon from '../components/Icon.svelte';
  import ScreenHeader from '../components/ScreenHeader.svelte';
  import { addMusicFolder, library } from '../library.svelte';
  import { formatDuration, player } from '../player.svelte';
  import { router } from '../router.svelte';

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
    : 'Welcome to PixelPlayer PC'}
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
        Pick the folder where your music lives (usually your Music folder). PixelPlayer reads it once, keeps up
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
</style>
