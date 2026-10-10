/**
 * Generated shapes in the spirit of Material 3 Expressive's shape library. Paths with the
 * same `points` count interpolate cleanly, so CSS can morph between them
 * (e.g. `clip-path: path(...)` transitions).
 */

function polar(cx: number, cy: number, r: number, a: number): [number, number] {
  return [cx + r * Math.cos(a), cy + r * Math.sin(a)];
}

function toPath(pts: [number, number][]): string {
  return `${pts.map(([x, y], i) => `${i ? 'L' : 'M'}${x.toFixed(2)} ${y.toFixed(2)}`).join(' ')} Z`;
}

/** A circle with `lobes` soft scallops ("cookie"); `depth` 0 gives a plain circle. */
export function scallopPath(size: number, lobes: number, depth: number, points = 144, rotation = 0): string {
  const c = size / 2;
  const base = c / (1 + depth);
  const pts: [number, number][] = [];
  for (let i = 0; i < points; i++) {
    const a = (i / points) * Math.PI * 2 + rotation;
    pts.push(polar(c, c, base * (1 + depth * Math.cos(lobes * (a - rotation))), a));
  }
  return toPath(pts);
}

/** A superellipse ("squircle"); exponent 2 is a circle, larger is squarer. */
export function squirclePath(size: number, exponent: number, points = 144): string {
  const c = size / 2;
  const pts: [number, number][] = [];
  for (let i = 0; i < points; i++) {
    const a = (i / points) * Math.PI * 2;
    const cos = Math.cos(a);
    const sin = Math.sin(a);
    const x = Math.sign(cos) * Math.abs(cos) ** (2 / exponent);
    const y = Math.sign(sin) * Math.abs(sin) ** (2 / exponent);
    pts.push([c + c * x, c + c * y]);
  }
  return toPath(pts);
}
