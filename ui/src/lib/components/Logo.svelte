<script lang="ts">
  import { scallopPath } from '../shapes';

  interface Props {
    size?: number;
  }
  let { size = 40 }: Props = $props();

  const outline = scallopPath(100, 9, 0.07);
</script>

<svg class="logo" width={size} height={size} viewBox="0 0 100 100" role="img" aria-label="Pixeltone">
  <path d={outline} class="shape" />
  <g class="bars">
    <rect x="29" y="40" width="10" height="20" rx="5" />
    <rect x="45" y="27" width="10" height="46" rx="5" />
    <rect x="61" y="36" width="10" height="28" rx="5" />
  </g>
</svg>

<style>
  .logo {
    flex: none;
  }
  .shape {
    fill: var(--md-sys-color-primary-container);
    transition: fill var(--motion-effects-slow-duration) var(--motion-effects-slow);
  }
  .bars {
    fill: var(--md-sys-color-on-primary-container);
  }
</style>
