<script lang="ts">
  import { type Bar, niceTicks } from '../stats';

  interface Props {
    /** One series: what it shows, for screen readers and the table caption. */
    name: string;
    data: readonly Bar[];
    format: (value: number) => string;
    /** Values on the y axis. */
    formatTick?: (value: number) => string;
    plotHeight?: number;
    /** Holding the previous render while new data loads. */
    loading?: boolean;
  }
  let { name, data, format, formatTick = format, plotHeight = 180, loading = false }: Props = $props();

  const LEFT = 48;
  const TOP = 20;
  const BOTTOM = 24;
  const MAX_BAR = 24;
  const GAP = 2;

  const uid = `bars-${Math.random().toString(36).slice(2, 8)}`;
  let width = $state(0);
  let active = $state<number | null>(null);
  let showTable = $state(false);

  const max = $derived(Math.max(0, ...data.map((d) => d.value)));
  const ticks = $derived(niceTicks(max));
  const top = $derived(ticks[ticks.length - 1] ?? 1);
  const plotWidth = $derived(Math.max(0, width - LEFT - 4));
  const slot = $derived(data.length ? plotWidth / data.length : 0);
  const barWidth = $derived(Math.max(1, Math.min(MAX_BAR, slot - GAP)));
  const peak = $derived(max > 0 ? data.findIndex((d) => d.value === max) : -1);

  const y = (v: number) => TOP + plotHeight - (top > 0 ? (v / top) * plotHeight : 0);
  const cx = (i: number) => LEFT + slot * i + slot / 2;

  /** A bar with a 4px rounded data end and a square base. */
  function barPath(i: number): string {
    const h = Math.max(0, TOP + plotHeight - y(data[i]!.value));
    if (h <= 0) return '';
    const x = cx(i) - barWidth / 2;
    const base = TOP + plotHeight;
    const r = Math.min(4, barWidth / 2, h);
    return `M${x},${base}V${base - h + r}Q${x},${base - h} ${x + r},${base - h}H${x + barWidth - r}Q${x + barWidth},${base - h} ${x + barWidth},${base - h + r}V${base}Z`;
  }

  function onkeydown(e: KeyboardEvent): void {
    if (!data.length) return;
    const last = data.length - 1;
    const next =
      e.key === 'ArrowRight' ? Math.min(last, (active ?? -1) + 1)
      : e.key === 'ArrowLeft' ? Math.max(0, (active ?? last + 1) - 1)
      : e.key === 'Home' ? 0
      : e.key === 'End' ? last
      : null;
    if (next === null) return;
    e.preventDefault();
    active = next;
  }

  const tip = $derived(active !== null && data[active] ? data[active] : null);
  const tipLeft = $derived(active === null ? 0 : Math.max(60, Math.min(width - 60, cx(active))));
</script>

<figure class={['chart', { loading }]}>
  <div
    class="plot"
    bind:clientWidth={width}
    role="listbox"
    aria-label="{name}. Use the arrow keys to read each bar."
    aria-activedescendant={active === null ? undefined : `${uid}-${active}`}
    tabindex="0"
    {onkeydown}
    onblur={() => (active = null)}
    onpointerleave={() => (active = null)}
  >
    {#if width > 0}
      <svg {width} height={TOP + plotHeight + BOTTOM}>
        {#each ticks as t (t)}
          <line class="grid" x1={LEFT} x2={width - 4} y1={y(t)} y2={y(t)} />
          <text class="tick" x={LEFT - 8} y={y(t)} dy="0.32em" text-anchor="end" aria-hidden="true">{formatTick(t)}</text>
        {/each}
        {#each data as d, i (i)}
          <path class={['bar', { active: active === i }]} d={barPath(i)} />
          {#if d.label}
            <text class="xlabel" x={cx(i)} y={TOP + plotHeight + 16} text-anchor="middle" aria-hidden="true">{d.label}</text>
          {/if}
          <rect
            id="{uid}-{i}"
            class="hit"
            x={LEFT + slot * i}
            y={TOP}
            width={slot}
            height={plotHeight}
            role="option"
            aria-selected={active === i}
            tabindex="-1"
            aria-label="{d.title}: {format(d.value)}"
            onpointerenter={() => (active = i)}
          />
        {/each}
        {#if peak >= 0 && data[peak]}
          <text class="peak" x={cx(peak)} y={y(max) - 6} text-anchor="middle" aria-hidden="true">{format(max)}</text>
        {/if}
      </svg>
    {/if}
    {#if tip}
      <div class="tooltip" style:left="{tipLeft}px" role="status">
        <strong class="label-large">{format(tip.value)}</strong>
        <span class="body-small">{tip.title}</span>
      </div>
    {/if}
  </div>
  <div class="foot">
    <button type="button" class="table-toggle label-medium" aria-expanded={showTable} onclick={() => (showTable = !showTable)}>
      {showTable ? 'Hide table' : 'Show as a table'}
    </button>
  </div>
  {#if showTable}
    <table class="body-small">
      <caption class="visually-hidden">{name}</caption>
      <tbody>
        {#each data as d, i (i)}
          <tr><th scope="row">{d.title}</th><td>{format(d.value)}</td></tr>
        {/each}
      </tbody>
    </table>
  {/if}
</figure>

<style>
  .chart {
    margin: 0;
    transition: opacity var(--motion-effects-default-duration) var(--motion-effects-default);
  }
  .chart.loading {
    opacity: 0.5;
  }
  .plot {
    position: relative;
    border-radius: var(--shape-sm);
  }
  .plot:focus-visible {
    outline: 2px solid var(--md-sys-color-secondary);
    outline-offset: 4px;
  }
  svg {
    display: block;
    overflow: visible;
  }
  .grid {
    stroke: var(--md-sys-color-outline-variant);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }
  .tick,
  .xlabel {
    fill: var(--md-sys-color-on-surface-variant);
    font: 500 11px/1 var(--font-plain);
    font-variant-numeric: tabular-nums;
  }
  .peak {
    fill: var(--md-sys-color-on-surface);
    font: 600 11px/1 var(--font-plain);
  }
  .bar {
    fill: var(--pp-chart-mark);
    transition: filter var(--motion-effects-fast-duration) var(--motion-effects-fast);
  }
  .bar.active {
    filter: brightness(1.18) saturate(1.1);
  }
  .hit {
    fill: transparent;
  }
  .tooltip {
    position: absolute;
    top: -8px;
    display: grid;
    gap: 2px;
    min-width: 96px;
    padding: 6px 10px;
    border-radius: var(--shape-sm);
    background: var(--md-sys-color-inverse-surface);
    color: var(--md-sys-color-inverse-on-surface);
    text-align: center;
    pointer-events: none;
    translate: -50% -100%;
    white-space: nowrap;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
  }
  .table-toggle {
    padding: 4px 8px;
    border: none;
    border-radius: var(--shape-full);
    background: none;
    color: var(--md-sys-color-primary);
    cursor: pointer;
  }
  .table-toggle:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, transparent);
  }
  table {
    width: 100%;
    margin-top: 8px;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 4px 8px;
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
    text-align: start;
    font-weight: normal;
  }
  td {
    text-align: end;
    font-variant-numeric: tabular-nums;
  }
</style>
