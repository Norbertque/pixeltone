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

import { formatDuration, formatShort } from './player.svelte';

describe('formatShort', () => {
  it('labels lossless, lossy and DSD', () => {
    const base = { codec: 'flac', sampleRate: 96_000, bits: 24, lossless: true, dsd: false };
    expect(formatShort(base)).toBe('FLAC 24/96');
    expect(formatShort({ ...base, sampleRate: 44_100, bits: 16 })).toBe('FLAC 16/44.1');
    expect(formatShort({ ...base, codec: 'mp3', lossless: false, bits: 0 })).toBe('MP3');
    expect(formatShort({ ...base, codec: 'dsd_lsbf_planar', sampleRate: 2_822_400, dsd: true })).toBe('DSD64');
  });

  it('formats durations', () => {
    expect(formatDuration(38 * 60_000)).toBe('38 min');
    expect(formatDuration(72 * 60_000)).toBe('1 h 12 min');
    expect(formatDuration(120 * 60_000)).toBe('2 h');
  });
});
