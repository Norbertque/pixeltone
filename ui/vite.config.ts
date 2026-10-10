import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

// The UI only ever runs in WebView2 (evergreen Chromium) or, for development, a current
// Chromium browser with the mock backend.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    target: 'chrome120',
    outDir: 'dist',
    emptyOutDir: true,
    chunkSizeWarningLimit: 1500,
  },
  test: {
    include: ['src/**/*.test.ts'],
    // material-color-utilities 0.4.0 has an extensionless ESM import that plain Node rejects;
    // letting Vite transform it resolves the import the same way the app build does.
    server: { deps: { inline: ['@material/material-color-utilities'] } },
  },
});
