/** Listening stats: time formatting, axis ticks and the bar series the charts draw. */
import type { DayListening } from './api';

export interface Bar {
  /** Axis label (empty to leave a gap). */
  label: string;
  /** Full name for the tooltip and the table. */
  title: string;
  value: number;
}

const MIN = 60_000;
const HOUR = 60 * MIN;

/** "3 h 12 min", "45 min", "under a minute". */
export function formatListening(ms: number): string {
  if (ms < MIN) return ms > 0 ? 'under a minute' : '0 min';
  const minutes = Math.round(ms / MIN);
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `${m} min`;
  return m ? `${h.toLocaleString()} h ${m} min` : `${h.toLocaleString()} h`;
}

/** Axis ticks: "0", "30 min", "2 h". */
export function formatTick(ms: number): string {
  if (ms === 0) return '0';
  if (ms < HOUR) return `${Math.round(ms / MIN)} min`;
  const h = ms / HOUR;
  return `${Number.isInteger(h) ? h : h.toFixed(1)} h`;
}

const STEPS = [1, 2, 5, 10, 15, 30].map((m) => m * MIN).concat([1, 2, 3, 6, 12, 24, 48, 96, 168].map((h) => h * HOUR));

/** Clean y-axis ticks from 0 to just above `max` (at most 5). */
export function niceTicks(max: number): number[] {
  if (max <= 0) return [0, 15 * MIN];
  const step = STEPS.find((s) => max / s <= 4) ?? Math.ceil(max / 4 / HOUR) * HOUR;
  const ticks = [];
  for (let t = 0; t < max + step; t += step) ticks.push(t);
  return ticks;
}

const pad = (n: number) => String(n).padStart(2, '0');
const dayKey = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/** The last `days` days up to `today`, one bar each (days without listening are 0). */
export function dailyBars(perDay: readonly DayListening[], days: number, today = new Date()): Bar[] {
  const byDay = new Map(perDay.map((d) => [d.day, d.ms]));
  const bars: Bar[] = [];
  for (let i = days - 1; i >= 0; i--) {
    const d = new Date(today.getFullYear(), today.getMonth(), today.getDate() - i);
    const label =
      days <= 7
        ? d.toLocaleDateString(undefined, { weekday: 'short' })
        : (days - 1 - i) % 5 === 0
          ? String(d.getDate())
          : '';
    bars.push({
      label,
      title: d.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' }),
      value: byDay.get(dayKey(d)) ?? 0,
    });
  }
  return bars;
}

/**
 * One bar per month: the last `months` months, or (when `months` is null) every month
 * from the first one with listening.
 */
export function monthlyBars(perDay: readonly DayListening[], months: number | null, today = new Date()): Bar[] {
  const byMonth = new Map<string, number>();
  for (const d of perDay) {
    const key = d.day.slice(0, 7);
    byMonth.set(key, (byMonth.get(key) ?? 0) + d.ms);
  }
  let count = months ?? 1;
  if (months === null && perDay.length) {
    const first = perDay.reduce((a, d) => (d.day < a ? d.day : a), perDay[0]!.day);
    const [y, m] = first.split('-').map(Number) as [number, number];
    count = Math.max(1, (today.getFullYear() - y) * 12 + today.getMonth() + 2 - m);
  }
  const every = count > 24 ? 6 : count > 12 ? 3 : 1;
  const bars: Bar[] = [];
  for (let i = count - 1; i >= 0; i--) {
    const d = new Date(today.getFullYear(), today.getMonth() - i, 1);
    const key = `${d.getFullYear()}-${pad(d.getMonth() + 1)}`;
    const index = count - 1 - i;
    const short = d.toLocaleDateString(undefined, { month: 'short' });
    bars.push({
      label: index % every === 0 ? (every > 1 || d.getMonth() === 0 ? `${short} ${String(d.getFullYear()).slice(2)}` : short) : '',
      title: d.toLocaleDateString(undefined, { month: 'long', year: 'numeric' }),
      value: byMonth.get(key) ?? 0,
    });
  }
  return bars;
}

/** Listening by hour of the day: 24 bars. */
export function hourBars(perHour: readonly number[]): Bar[] {
  return Array.from({ length: 24 }, (_, h) => ({
    label: h % 3 === 0 ? String(h) : '',
    title: `${pad(h)}:00–${pad((h + 1) % 24)}:00`,
    value: perHour[h] ?? 0,
  }));
}
