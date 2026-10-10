import { describe, expect, it } from 'vitest';

import { formatTime } from './player.svelte';

describe('formatTime', () => {
  it('formats minutes and hours', () => {
    expect(formatTime(0)).toBe('0:00');
    expect(formatTime(67_400)).toBe('1:07');
    expect(formatTime(3_723_000)).toBe('1:02:03');
    expect(formatTime(-5)).toBe('0:00');
  });
});
