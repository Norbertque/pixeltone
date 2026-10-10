<script lang="ts">
  import Button from '../components/Button.svelte';
  import { player } from '../player.svelte';
  import Icon from '../components/Icon.svelte';
  import ScreenHeader from '../components/ScreenHeader.svelte';
  import { router } from '../router.svelte';

  const hour = new Date().getHours();
  const greeting = hour < 5 ? 'Good night' : hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening';

  const cards = [
    {
      icon: 'library_add',
      tone: 'primary',
      title: 'Add your music',
      body: 'Point PixelPlayer at your music folders. Library scanning arrives in an upcoming update.',
      action: 'Library settings',
    },
    {
      icon: 'graphic_eq',
      tone: 'tertiary',
      title: 'Bit-perfect output',
      body: 'WASAPI exclusive mode sends your files to the DAC untouched, at their own sample rate and bit depth.',
      action: 'Audio output',
    },
    {
      icon: 'palette',
      tone: 'secondary',
      title: 'Make it yours',
      body: 'Colours follow the album art, or pick your own seed colour and palette style.',
      action: 'Appearance',
    },
  ] as const;
</script>

<ScreenHeader title={greeting} subtitle="Welcome to PixelPlayer PC">
  {#snippet actions()}
    <Button variant="filled" size="md" icon="play_arrow" onclick={() => player.openFiles()}>Play files…</Button>
  {/snippet}
</ScreenHeader>

<div class="cards">
  {#each cards as card (card.title)}
    <article class={['card', card.tone]}>
      <div class="icon"><Icon name={card.icon} size={28} /></div>
      <h2 class="title-large rounded emphasized">{card.title}</h2>
      <p class="body-medium">{card.body}</p>
      <div class="action">
        <Button variant="text" icon="arrow_forward" onclick={() => router.go('settings')}>{card.action}</Button>
      </div>
    </article>
  {/each}
</div>

<style>
  .cards {
    container: cards / inline-size;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 16px;
  }
  .card {
    display: grid;
    grid-template-rows: auto auto 1fr auto;
    gap: 8px;
    padding: 24px 24px 12px;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-low);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    margin-bottom: 8px;
    border-radius: var(--shape-lg);
  }
  .primary .icon {
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }
  .secondary .icon {
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .tertiary .icon {
    background: var(--md-sys-color-tertiary-container);
    color: var(--md-sys-color-on-tertiary-container);
  }
  h2 {
    margin: 0;
  }
  p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }
  .action {
    margin-left: -12px;
  }
</style>
