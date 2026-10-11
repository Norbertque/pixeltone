import '@fontsource-variable/google-sans-flex/full.css';
import 'material-symbols/rounded.css';
import './app.css';

import { mount } from 'svelte';

import App from './App.svelte';
import { api } from './lib/api';
import { initLibrary } from './lib/library.svelte';
import { initPlayer } from './lib/player.svelte';
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

void initPlayer();
void initLibrary();

const target = document.getElementById('app');
if (!target) throw new Error('#app missing');

export default mount(App, { target });
