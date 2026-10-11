<script lang="ts">
  import { fly } from 'svelte/transition';

  import MiniPlayer from './lib/components/MiniPlayer.svelte';
  import NavBar from './lib/components/NavBar.svelte';
  import NavRail from './lib/components/NavRail.svelte';
  import NowPlaying from './lib/components/NowPlaying.svelte';
  import Snackbar from './lib/components/Snackbar.svelte';
  import { player } from './lib/player.svelte';
  import { router } from './lib/router.svelte';
  import About from './lib/screens/About.svelte';
  import AlbumScreen from './lib/screens/AlbumScreen.svelte';
  import ArtistScreen from './lib/screens/ArtistScreen.svelte';
  import FolderScreen from './lib/screens/FolderScreen.svelte';
  import GenreScreen from './lib/screens/GenreScreen.svelte';
  import Home from './lib/screens/Home.svelte';
  import Library from './lib/screens/Library.svelte';
  import Search from './lib/screens/Search.svelte';
  import Settings from './lib/screens/Settings.svelte';
  import { theme } from './lib/theme/theme.svelte';

  const SCREENS = { home: Home, search: Search, library: Library, settings: Settings, about: About };
  const route = $derived(router.route);
  const routeKey = $derived(JSON.stringify(router.route));

  // The playing album's colour drives the whole theme.
  $effect(() => {
    theme.artworkSeed = player.artworkSeed;
  });

  function onkeydown(e: KeyboardEvent): void {
    const target = e.target as HTMLElement | null;
    const typing = target?.closest('input, textarea, [contenteditable="true"]');
    if (e.altKey && e.key === 'ArrowLeft') {
      e.preventDefault();
      router.back();
    } else if (e.ctrlKey && e.key.toLowerCase() === 'o') {
      e.preventDefault();
      void player.openFiles();
    } else if (e.key === ' ' && !typing && !target?.closest('button, [role="slider"]')) {
      e.preventDefault();
      player.togglePlay();
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="app">
  <div class="rail-slot">
    <NavRail />
  </div>

  <div class="main-col">
    <main class="content scroller" inert={router.nowPlayingOpen}>
      {#key routeKey}
        <div class="screen" in:fly={{ y: 16, duration: 260, opacity: 0 }}>
          {#if route.screen === 'album'}
            <AlbumScreen id={route.id} />
          {:else if route.screen === 'artist'}
            <ArtistScreen id={route.id} />
          {:else if route.screen === 'genre'}
            <GenreScreen id={route.id} />
          {:else if route.screen === 'folder'}
            <FolderScreen path={route.path} />
          {:else}
            {@const Screen = SCREENS[route.screen]}
            <Screen />
          {/if}
        </div>
      {/key}
    </main>

    <div class="player-slot" inert={router.nowPlayingOpen}>
      <MiniPlayer />
    </div>

    <div class="bar-slot">
      <NavBar />
    </div>
  </div>

  {#if router.nowPlayingOpen}
    <NowPlaying />
  {/if}

  <Snackbar />
</div>

<style>
  .app {
    container: app / size;
    position: relative;
    display: grid;
    grid-template-columns: auto 1fr;
    height: 100%;
    background: var(--md-sys-color-surface-container);
  }

  .main-col {
    display: grid;
    grid-template-rows: 1fr auto auto;
    min-width: 0;
    min-height: 0;
  }

  .content {
    min-height: 0;
    margin: 12px 12px 0 0;
    padding: 0 clamp(16px, 4cqi, 48px);
    border-radius: var(--shape-xl) var(--shape-xl) 0 0;
    background: var(--md-sys-color-surface);
  }
  .screen {
    max-width: 1280px;
    margin: 0 auto;
  }

  .player-slot {
    padding: 12px 12px 12px 0;
  }
  .player-slot:empty {
    display: none;
  }

  .bar-slot {
    display: none;
  }

  @container app (width < 640px) {
    .app {
      grid-template-columns: 1fr;
    }
    .rail-slot {
      display: none;
    }
    .bar-slot {
      display: block;
    }
    .content {
      margin: 0;
      border-radius: 0;
    }
    .player-slot {
      padding: 8px;
    }
  }
</style>
