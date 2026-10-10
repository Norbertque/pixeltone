<script lang="ts">
  import { onDestroy } from 'svelte';

  interface Props {
    /** 0…1, used when no live getter is given and while paused. */
    value: number;
    /** Reads the interpolated value each animation frame while playing. */
    getValue?: () => number;
    playing?: boolean;
    onseek?: (value: number) => void;
    label: string;
    valueText?: string;
    /** Keyboard step as a fraction of the whole. */
    step?: number;
    thickness?: number;
    amplitude?: number;
    wavelength?: number;
    height?: number;
    disabled?: boolean;
  }

  let {
    value,
    getValue,
    playing = false,
    onseek,
    label,
    valueText,
    step = 0.02,
    thickness = 4,
    amplitude = 3,
    wavelength = 34,
    height = 32,
    disabled = false,
  }: Props = $props();

  let width = $state(0);
  let dragging = $state(false);
  let dragValue = 0;

  let activePath: SVGPathElement | undefined = $state();
  let inactivePath: SVGPathElement | undefined = $state();
  let thumb: SVGRectElement | undefined = $state();

  let phase = 0;
  let amp = 0;
  let raf = 0;
  let last = 0;

  const GAP = 6;
  const THUMB_W = 4;

  function current(): number {
    const v = dragging ? dragValue : playing && getValue ? getValue() : value;
    return Math.min(1, Math.max(0, Number.isFinite(v) ? v : 0));
  }

  function draw(): void {
    if (!activePath || !inactivePath || !thumb || width <= 0) return;
    const pad = thickness / 2 + amplitude;
    const mid = height / 2;
    const span = Math.max(0, width - pad * 2);
    const x = pad + span * current();

    // Active part: a sine wave whose amplitude eases to 0 when paused.
    const end = Math.max(pad, x - GAP - THUMB_W / 2);
    let d = '';
    if (end > pad) {
      const k = (Math.PI * 2) / wavelength;
      for (let px = pad; px <= end; px += 2) {
        const y = mid + amp * Math.sin(px * k - phase);
        d += `${d ? 'L' : 'M'}${px.toFixed(1)} ${y.toFixed(2)}`;
      }
      const yEnd = mid + amp * Math.sin(end * k - phase);
      d += `L${end.toFixed(1)} ${yEnd.toFixed(2)}`;
    }
    activePath.setAttribute('d', d);

    const start = Math.min(width - pad, x + GAP + THUMB_W / 2);
    inactivePath.setAttribute('d', start < width - pad ? `M${start.toFixed(1)} ${mid}L${(width - pad).toFixed(1)} ${mid}` : '');

    const th = height * 0.72;
    thumb.setAttribute('x', (x - THUMB_W / 2).toFixed(1));
    thumb.setAttribute('y', (mid - th / 2).toFixed(1));
    thumb.setAttribute('height', th.toFixed(1));
  }

  function frame(now: number): void {
    const dt = last ? Math.min(0.05, (now - last) / 1000) : 0;
    last = now;
    const target = playing && !dragging ? amplitude : 0;
    // Critically damped approach (~150 ms).
    amp += (target - amp) * (1 - Math.exp(-dt * 14));
    if (Math.abs(target - amp) < 0.01) amp = target;
    if (amp > 0) phase += dt * 7;
    draw();
    if (playing || dragging || amp > 0) {
      raf = requestAnimationFrame(frame);
    } else {
      raf = 0;
      last = 0;
    }
  }

  function kick(): void {
    if (!raf) {
      last = 0;
      raf = requestAnimationFrame(frame);
    }
  }

  // Redraw on any input change; animate while playing.
  $effect(() => {
    void [value, playing, width, activePath, amplitude, wavelength, thickness];
    draw();
    if (playing || amp > 0) kick();
  });

  onDestroy(() => cancelAnimationFrame(raf));

  let root: HTMLDivElement | undefined = $state();

  function valueAt(clientX: number): number {
    if (!root) return 0;
    const rect = root.getBoundingClientRect();
    const pad = thickness / 2 + amplitude;
    return Math.min(1, Math.max(0, (clientX - rect.left - pad) / Math.max(1, rect.width - pad * 2)));
  }

  function onpointerdown(e: PointerEvent): void {
    if (disabled || e.button !== 0) return;
    root?.setPointerCapture(e.pointerId);
    dragging = true;
    dragValue = valueAt(e.clientX);
    kick();
  }

  function onpointermove(e: PointerEvent): void {
    if (!dragging) return;
    dragValue = valueAt(e.clientX);
  }

  function onpointerup(e: PointerEvent): void {
    if (!dragging) return;
    dragging = false;
    dragValue = valueAt(e.clientX);
    onseek?.(dragValue);
  }

  function onkeydown(e: KeyboardEvent): void {
    if (disabled) return;
    const v = current();
    const moves: Record<string, number> = {
      ArrowLeft: v - step,
      ArrowDown: v - step,
      ArrowRight: v + step,
      ArrowUp: v + step,
      PageDown: v - step * 5,
      PageUp: v + step * 5,
      Home: 0,
    };
    const next = moves[e.key];
    if (next === undefined) return;
    e.preventDefault();
    onseek?.(Math.min(1, Math.max(0, next)));
  }
</script>

<div
  bind:this={root}
  bind:clientWidth={width}
  class={['wavy', { dragging, disabled }]}
  style:height="{height}px"
  role="slider"
  tabindex={disabled ? -1 : 0}
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={1000}
  aria-valuenow={Math.round(value * 1000)}
  aria-valuetext={valueText}
  aria-disabled={disabled}
  {onpointerdown}
  {onpointermove}
  {onpointerup}
  onpointercancel={() => (dragging = false)}
  {onkeydown}
>
  <svg width={width} {height} aria-hidden="true">
    <path bind:this={inactivePath} class="inactive" stroke-width={thickness} />
    <path bind:this={activePath} class="active" stroke-width={thickness} />
    <rect bind:this={thumb} class="thumb" width={THUMB_W} rx={THUMB_W / 2} />
  </svg>
</div>

<style>
  .wavy {
    position: relative;
    width: 100%;
    cursor: pointer;
    touch-action: none;
    border-radius: var(--shape-sm);
  }
  .disabled {
    cursor: default;
    opacity: 0.38;
  }
  svg {
    display: block;
    overflow: visible;
  }
  path {
    fill: none;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .active {
    stroke: var(--md-sys-color-primary);
  }
  .inactive {
    stroke: var(--md-sys-color-secondary-container);
  }
  .thumb {
    fill: var(--md-sys-color-primary);
  }
  .dragging .thumb {
    fill: var(--md-sys-color-on-primary-container);
  }
</style>
