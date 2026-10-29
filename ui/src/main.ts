import '@fontsource-variable/google-sans-flex/full.css';
import 'material-symbols/rounded.css';
import './app.css';

import { mount } from 'svelte';

import App from './App.svelte';
import MiniWindow from './MiniWindow.svelte';
import { api } from './lib/api';
import { initJam } from './lib/jam.svelte';
import { initLibrary } from './lib/library.svelte';
import { output } from './lib/output.svelte';
import { initPlayer } from './lib/player.svelte';
import { initUpdates } from './lib/updates.svelte';
import { installMotionTokens } from './lib/theme/motion';
import { initTheme } from './lib/theme/theme.svelte';

installMotionTokens();

let captionTimer: ReturnType<typeof setTimeout> | undefined;
initTheme((surface, dark) => {
  // Debounced: the theme transitions over several hundred milliseconds.
  clearTimeout(captionTimer);
  captionTimer = setTimeout(() => {
    api.setWindowTheme(surface, dark).catch(() => {});
  }, 120);
});

// The mini player is the same page in a small window of its own.
const mini = location.hash === '#mini';

void initPlayer();
if (!mini) {
  void initLibrary();
  void output.start();
  void initJam();
  initUpdates();
}

const target = document.getElementById('app');
if (!target) throw new Error('#app missing');

export default mount(mini ? MiniWindow : App, { target });
