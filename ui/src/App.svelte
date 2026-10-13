<script lang="ts">
  import { fly } from 'svelte/transition';

  import ContextMenu from './lib/components/ContextMenu.svelte';
  import MiniPlayer from './lib/components/MiniPlayer.svelte';
  import PlaylistPicker from './lib/components/PlaylistPicker.svelte';
  import PromptDialog from './lib/components/PromptDialog.svelte';
  import NavBar from './lib/components/NavBar.svelte';
  import NavRail from './lib/components/NavRail.svelte';
  import NowPlaying from './lib/components/NowPlaying.svelte';
  import ShortcutsDialog from './lib/components/ShortcutsDialog.svelte';
  import Snackbar from './lib/components/Snackbar.svelte';
  import { help } from './lib/help.svelte';
  import { output } from './lib/output.svelte';
  import { player } from './lib/player.svelte';
  import { router } from './lib/router.svelte';
  import About from './lib/screens/About.svelte';
  import AlbumScreen from './lib/screens/AlbumScreen.svelte';
  import ArtistScreen from './lib/screens/ArtistScreen.svelte';
  import FavoritesScreen from './lib/screens/FavoritesScreen.svelte';
  import FolderScreen from './lib/screens/FolderScreen.svelte';
  import GenreScreen from './lib/screens/GenreScreen.svelte';
  import Home from './lib/screens/Home.svelte';
  import Library from './lib/screens/Library.svelte';
  import PlaylistScreen from './lib/screens/PlaylistScreen.svelte';
  import Search from './lib/screens/Search.svelte';
  import Settings from './lib/screens/Settings.svelte';
  import StatsScreen from './lib/screens/StatsScreen.svelte';
  import { type ShortcutAction, shortcutFor } from './lib/shortcuts';
  import { nudge } from './lib/volume';
  import { theme } from './lib/theme/theme.svelte';

  const SCREENS = {
    home: Home,
    search: Search,
    library: Library,
    settings: Settings,
    about: About,
    favorites: FavoritesScreen,
    stats: StatsScreen,
  };
  const route = $derived(router.route);
  const routeKey = $derived(JSON.stringify(router.route));

  // The playing album's colour drives the whole theme.
  $effect(() => {
    theme.artworkSeed = player.artworkSeed;
  });

  const SEEK_STEP_MS = 10_000;

  function run(action: ShortcutAction): void {
    switch (action) {
      case 'toggle':
        return player.togglePlay();
      case 'next':
        return player.next();
      case 'previous':
        return player.previous();
      case 'seekForward':
        return player.seekBy(SEEK_STEP_MS);
      case 'seekBack':
        return player.seekBy(-SEEK_STEP_MS);
      case 'shuffle':
        return player.toggleShuffle();
      case 'repeat':
        return player.cycleRepeat();
      case 'volumeUp':
      case 'volumeDown': {
        const v = output.volume;
        if (v && v.kind !== 'fixed') output.setVolume(nudge({ ...v, db: output.draft ?? v.db }, action === 'volumeUp' ? 1 : -1));
        return;
      }
      case 'mute':
        return output.toggleMute();
      case 'queue':
        return void router.togglePanel('queue');
      case 'search':
        if (router.screen === 'search') document.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
        else void router.go('search');
        return;
      case 'library':
        return void router.go('library');
      case 'settings':
        return void router.go('settings');
      case 'openFiles':
        return void player.openFiles();
      case 'back':
        return void router.back();
      case 'help':
        help.shortcuts = true;
        return;
    }
  }

  function onkeydown(e: KeyboardEvent): void {
    // Menus, sliders and dialogs handle their own keys first.
    if (e.defaultPrevented || document.querySelector('dialog:modal')) return;
    const target = e.target instanceof HTMLElement ? e.target : null;
    const action = shortcutFor(e, {
      typing: !!target?.closest(
        'input:not([type="range"], [type="checkbox"], [type="radio"], [type="color"]), textarea, select, [contenteditable="true"]',
      ),
      activates: !!target?.closest('button, a[href], summary, [role="switch"], [role="menuitem"], [role="slider"]'),
      slider: !!target?.closest('[role="slider"], input[type="range"]'),
    });
    if (!action) return;
    e.preventDefault();
    run(action);
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
          {:else if route.screen === 'playlist'}
            <PlaylistScreen id={route.id} />
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
  <ContextMenu />
  <ShortcutsDialog />
  <PromptDialog />
  <PlaylistPicker />
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
