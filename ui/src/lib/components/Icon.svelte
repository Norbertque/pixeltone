<script lang="ts">
  interface Props {
    /** Material Symbols ligature name, e.g. "play_arrow". */
    name: string;
    size?: number;
    fill?: boolean;
    weight?: number;
    class?: string;
  }

  let { name, size = 24, fill = false, weight = 400, class: className = '' }: Props = $props();

  const opsz = $derived(Math.min(48, Math.max(20, size)));
</script>

<span
  class={['icon', 'material-symbols-rounded', className]}
  aria-hidden="true"
  style:font-size="{size}px"
  style:font-variation-settings="'FILL' {fill ? 1 : 0}, 'wght' {weight}, 'GRAD' 0, 'opsz' {opsz}"
>
  {name}
</span>

<style>
  .icon {
    display: inline-block;
    flex: none;
    width: 1em;
    height: 1em;
    overflow: hidden;
    line-height: 1;
    text-align: center;
    user-select: none;
    transition: font-variation-settings var(--motion-effects-default-duration) var(--motion-effects-default);
  }
</style>
