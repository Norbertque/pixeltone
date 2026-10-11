/**
 * Pure windowing math for virtual lists and grids.
 *
 * `listTop` is the list's top edge relative to the top of the scroll viewport: 0 when the
 * list starts at the top of the view, negative once it has been scrolled past.
 */
export function visibleRange(
  listTop: number,
  viewHeight: number,
  rowHeight: number,
  count: number,
  overscan: number,
): [number, number] {
  if (count <= 0 || rowHeight <= 0) return [0, 0];
  const first = Math.floor(-listTop / rowHeight);
  const visible = Math.ceil(viewHeight / rowHeight) + 1;
  const start = Math.max(0, Math.min(count, first - overscan));
  const end = Math.max(start, Math.min(count, first + visible + overscan));
  return [start, end];
}

/** Columns that fit `width` with items at least `minItem` wide, and the resulting item width. */
export function gridColumns(width: number, minItem: number, gap: number): { cols: number; itemWidth: number } {
  const cols = Math.max(1, Math.floor((width + gap) / (minItem + gap)));
  const itemWidth = Math.max(0, (width - gap * (cols - 1)) / cols);
  return { cols, itemWidth };
}

/** Finds the element's scrolling ancestor (the app's main content area). */
export function scrollParent(el: HTMLElement): HTMLElement | null {
  return el.closest<HTMLElement>('.scroller');
}
