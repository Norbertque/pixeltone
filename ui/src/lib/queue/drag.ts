/**
 * Drag-to-reorder maths for fixed-height rows. While row `from` is dragged, it follows the
 * pointer and the rows it passes slide one row up or down to show where it will land.
 */

/** Where the dragged row's top edge may go: anywhere from the first to the last slot. */
export function clampTop(top: number, rowHeight: number, count: number): number {
  return Math.max(0, Math.min(Math.max(0, count - 1) * rowHeight, top));
}

/** The index a row dragged to `top` (px from the list's top) lands on. */
export function dropIndex(top: number, rowHeight: number, count: number): number {
  if (count <= 0) return 0;
  return Math.max(0, Math.min(count - 1, Math.round(top / rowHeight)));
}

/** How far row `i` slides while row `from` is dragged over index `to`. */
export function displacement(i: number, from: number, to: number, rowHeight: number): number {
  if (from < to && i > from && i <= to) return -rowHeight;
  if (to < from && i >= to && i < from) return rowHeight;
  return 0;
}

/**
 * Auto-scroll speed (px per frame) for a pointer at `y` in a viewport spanning
 * `top..bottom`: faster the deeper it is in an `edge`-px band at either end.
 */
export function edgeScroll(y: number, top: number, bottom: number, edge = 56, max = 18): number {
  if (y < top + edge) return -Math.ceil(max * Math.min(1, (top + edge - y) / edge));
  if (y > bottom - edge) return Math.ceil(max * Math.min(1, (y - (bottom - edge)) / edge));
  return 0;
}
