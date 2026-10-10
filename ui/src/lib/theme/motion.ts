/**
 * Material 3 Expressive motion: physical springs, turned into CSS `linear()` easings so
 * plain transitions move like the Compose motion scheme.
 *
 * Spatial springs (position, size, shape) overshoot slightly; effects springs (colour,
 * opacity) are critically damped.
 */

export interface Spring {
  stiffness: number;
  dampingRatio: number;
}

export const SPRINGS = {
  'spatial-fast': { stiffness: 1400, dampingRatio: 0.9 },
  'spatial-default': { stiffness: 700, dampingRatio: 0.9 },
  'spatial-slow': { stiffness: 300, dampingRatio: 0.9 },
  'effects-fast': { stiffness: 3800, dampingRatio: 1 },
  'effects-default': { stiffness: 1600, dampingRatio: 1 },
  'effects-slow': { stiffness: 800, dampingRatio: 1 },
} as const satisfies Record<string, Spring>;

export type SpringToken = keyof typeof SPRINGS;

/** Position of a unit-mass spring released from 0 towards 1 at time `t` seconds. */
export function springPosition({ stiffness, dampingRatio }: Spring, t: number): number {
  const w0 = Math.sqrt(stiffness);
  if (dampingRatio < 1) {
    const wd = w0 * Math.sqrt(1 - dampingRatio * dampingRatio);
    const decay = Math.exp(-dampingRatio * w0 * t);
    return 1 - decay * (Math.cos(wd * t) + ((dampingRatio * w0) / wd) * Math.sin(wd * t));
  }
  return 1 - Math.exp(-w0 * t) * (1 + w0 * t);
}

/** Time until the spring stays within `epsilon` of its target. */
export function springDuration(spring: Spring, epsilon = 0.001): number {
  const step = 1 / 600;
  let settledSince = 0;
  for (let t = 0; t < 3; t += step) {
    if (Math.abs(1 - springPosition(spring, t)) > epsilon) settledSince = t + step;
  }
  return settledSince;
}

/** A CSS `linear()` easing that follows the spring, and its duration in ms. */
export function springEasing(spring: Spring, samples = 48): { easing: string; durationMs: number } {
  const duration = springDuration(spring);
  const points: string[] = [];
  for (let i = 0; i <= samples; i++) {
    const p = i / samples;
    const v = i === samples ? 1 : springPosition(spring, p * duration);
    points.push(Number(v.toFixed(4)).toString());
  }
  return { easing: `linear(${points.join(', ')})`, durationMs: Math.round(duration * 1000) };
}

/** Publishes every spring as `--motion-<token>` (easing) and `--motion-<token>-duration`. */
export function installMotionTokens(root: HTMLElement = document.documentElement): void {
  for (const [name, spring] of Object.entries(SPRINGS)) {
    const { easing, durationMs } = springEasing(spring);
    root.style.setProperty(`--motion-${name}`, easing);
    root.style.setProperty(`--motion-${name}-duration`, `${durationMs}ms`);
  }
}
