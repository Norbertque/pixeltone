<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    src: string | null;
    alt?: string;
    size?: string;
    radius?: string;
    /** `view-transition-name` for shared-element transitions. */
    transitionName?: string;
  }
  let { src, alt = '', size = '56px', radius = 'var(--shape-md)', transitionName }: Props = $props();

  let failed = $state(false);
  $effect(() => {
    void src;
    failed = false;
  });
</script>

<div
  class="art"
  style:width={size}
  style:border-radius={radius}
  style:view-transition-name={transitionName}
>
  {#if src && !failed}
    <img {src} {alt} draggable="false" decoding="async" onerror={() => (failed = true)} />
  {:else}
    <div class="placeholder" role={alt ? 'img' : undefined} aria-label={alt || undefined}>
      <Icon name="music_note" size={28} />
    </div>
  {/if}
</div>

<style>
  .art {
    flex: none;
    aspect-ratio: 1;
    overflow: hidden;
    background: var(--md-sys-color-surface-container-highest);
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .placeholder {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    background: linear-gradient(
      135deg,
      var(--md-sys-color-primary-container),
      var(--md-sys-color-tertiary-container)
    );
    color: var(--md-sys-color-on-primary-container);
  }
</style>
