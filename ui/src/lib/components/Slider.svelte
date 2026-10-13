<script lang="ts">
  interface Props {
    value: number;
    min: number;
    max: number;
    step?: number;
    label: string;
    valueText?: string;
    disabled?: boolean;
    /** While dragging. */
    oninput?: (value: number) => void;
    /** On release. */
    onchange?: (value: number) => void;
  }
  let { value, min, max, step = 1, label, valueText, disabled = false, oninput, onchange }: Props = $props();

  const fraction = $derived(max > min ? (Math.min(max, Math.max(min, value)) - min) / (max - min) : 0);
</script>

<input
  class="slider"
  type="range"
  {min}
  {max}
  {step}
  {value}
  {disabled}
  aria-label={label}
  aria-valuetext={valueText}
  style:--fraction={fraction}
  oninput={(e) => oninput?.(Number(e.currentTarget.value))}
  onchange={(e) => onchange?.(Number(e.currentTarget.value))}
/>

<style>
  .slider {
    --track: 16px;
    width: 100%;
    height: 44px;
    margin: 0;
    background: transparent;
    cursor: pointer;
    appearance: none;
  }
  .slider::-webkit-slider-runnable-track {
    height: var(--track);
    border-radius: var(--shape-full);
    background: linear-gradient(
      to right,
      var(--md-sys-color-primary) 0 calc(var(--fraction) * 100%),
      var(--md-sys-color-secondary-container) calc(var(--fraction) * 100%) 100%
    );
  }
  .slider::-webkit-slider-thumb {
    width: 4px;
    height: 44px;
    margin-top: calc((var(--track) - 44px) / 2);
    border: none;
    border-radius: 2px;
    background: var(--md-sys-color-primary);
    box-shadow:
      -6px 0 0 var(--slider-surface, var(--md-sys-color-surface-container-low)),
      6px 0 0 var(--slider-surface, var(--md-sys-color-surface-container-low));
    appearance: none;
  }
  .slider:disabled {
    cursor: default;
    opacity: 0.38;
  }
  .slider:focus-visible {
    outline: none;
  }
  .slider:focus-visible::-webkit-slider-thumb {
    outline: 2px solid var(--md-sys-color-secondary);
    outline-offset: 2px;
  }
</style>
