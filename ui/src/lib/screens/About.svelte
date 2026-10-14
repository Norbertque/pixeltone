<script lang="ts">
  import { onMount } from 'svelte';

  import { api, type AppInfo } from '../api';
  import Button from '../components/Button.svelte';
  import Logo from '../components/Logo.svelte';
  import ScreenHeader from '../components/ScreenHeader.svelte';
  import { updates } from '../updates.svelte';

  let info = $state<AppInfo | null>(null);
  let error = $state<string | null>(null);

  onMount(() => {
    api
      .appInfo()
      .then((i) => (info = i))
      .catch((e: unknown) => (error = String(e)));
  });

  const CREDITS = [
    { name: 'FFmpeg', license: 'LGPL-2.1-or-later', use: 'Decoding' },
    { name: 'Tauri', license: 'MIT / Apache-2.0', use: 'App shell' },
    { name: 'Svelte', license: 'MIT', use: 'User interface' },
    { name: 'Material Color Utilities', license: 'Apache-2.0', use: 'Dynamic colour' },
    { name: 'Google Sans Flex', license: 'SIL OFL 1.1', use: 'Typeface' },
    { name: 'Material Symbols', license: 'Apache-2.0', use: 'Icons' },
    { name: 'LRCLIB', license: 'Online service', use: 'Synced lyrics' },
    { name: 'Deezer', license: 'Online service', use: 'Artist pictures' },
  ];
</script>

<ScreenHeader title="About" />

<div class="about">
  <section class="hero">
    <Logo size={96} />
    <div>
      <h2 class="headline-medium rounded emphasized">Pixeltone</h2>
      <p class="body-large">A Material 3 Expressive music player for Windows with bit-perfect WASAPI exclusive output.</p>
    </div>
  </section>

  <section class="card">
    <dl>
      <dt class="label-large">Version</dt>
      <dd class="body-large">{info?.appVersion ?? '…'}</dd>
      <dt class="label-large">FFmpeg</dt>
      <dd class="body-large">{info?.ffmpegVersion ?? '…'}</dd>
      <dt class="label-large">FFmpeg licence</dt>
      <dd class="body-large">{info?.ffmpegLicense ?? '…'}</dd>
      <dt class="label-large">Platform</dt>
      <dd class="body-large">{info?.os ?? '…'}</dd>
    </dl>
    {#if error}<p class="error body-medium" role="alert">{error}</p>{/if}
    <div class="update">
      {#if updates.installing}
        <span class="body-medium">
          Installing {updates.available?.version ?? 'the update'}{updates.progress !== null
            ? ` (${Math.round(updates.progress * 100)}%)`
            : '…'}
        </span>
      {:else if updates.available}
        <span class="body-medium">Version {updates.available.version} is available.</span>
        <Button variant="filled" icon="system_update_alt" onclick={() => void updates.install()}>Update and restart</Button>
      {:else}
        <span class="body-medium muted">{updates.upToDate ? 'This is the newest version.' : ''}</span>
        <Button variant="tonal" icon="update" disabled={updates.checking} onclick={() => void updates.check(true)}>
          {updates.checking ? 'Checking…' : 'Check for updates'}
        </Button>
      {/if}
    </div>
  </section>

  <section class="card">
    <h3 class="title-medium">Open-source software</h3>
    <ul>
      {#each CREDITS as c (c.name)}
        <li>
          <span class="body-large">{c.name}</span>
          <span class="body-medium muted">{c.use} · {c.license}</span>
        </li>
      {/each}
    </ul>
    <p class="body-medium muted">
      Inspired by PixelPlayer for Android. Pixeltone is an independent project and shares no code with it.
    </p>
  </section>
</div>

<style>
  .about {
    display: grid;
    gap: 16px;
    max-width: 760px;
    padding-bottom: 24px;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 8px 0 8px;
  }
  .hero h2 {
    margin: 0;
  }
  .hero p {
    margin: 4px 0 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .card {
    padding: 20px 24px;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-low);
    user-select: text;
  }
  .update {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--md-sys-color-outline-variant);
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 10px 24px;
    margin: 0;
  }
  dt {
    color: var(--md-sys-color-on-surface-variant);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  h3 {
    margin: 0 0 8px;
  }
  ul {
    display: grid;
    gap: 8px;
    margin: 0 0 16px;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 4px 16px;
  }
  .muted {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .error {
    margin: 12px 0 0;
    color: var(--md-sys-color-error);
  }
</style>
