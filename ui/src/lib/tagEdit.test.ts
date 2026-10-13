import { describe, expect, it } from 'vitest';

import type { TagFields } from './api';
import { changeFrom, FIELDS, hasChanges, initialForm, invalid, shared } from './tagEdit';

function fields(over: Partial<TagFields>): TagFields {
  return {
    title: 'Song',
    artist: 'Loser64',
    album: 'Misatocore',
    albumArtist: '',
    genre: 'Breakcore',
    year: '2021',
    trackNo: '1',
    discNo: '',
    ...over,
  };
}

const ALL = FIELDS.map((f) => f.key);

describe('tag editor', () => {
  it('shows what the songs share and leaves the rest blank', () => {
    const s = shared([fields({ title: 'A', trackNo: '1' }), fields({ title: 'B', trackNo: '2' })]);
    expect(s.title).toBeNull();
    expect(s.trackNo).toBeNull();
    expect(s.album).toBe('Misatocore');
    expect(initialForm(s)).toMatchObject({ title: '', album: 'Misatocore', discNo: '' });
  });

  it('writes only what changed', () => {
    const s = shared([fields({})]);
    const form = { ...initialForm(s), album: 'Misatocore, Vol. 2', genre: ' Breakcore ' };
    const change = changeFrom(s, form, ALL);
    expect(change.album).toBe('Misatocore, Vol. 2');
    expect(change.genre).toBeNull();
    expect(change.title).toBeNull();
    expect(hasChanges(change)).toBe(true);
    expect(hasChanges(changeFrom(s, initialForm(s), ALL))).toBe(false);
  });

  it('clears a shared field that was emptied, but never a differing one left blank', () => {
    const s = shared([fields({ title: 'A' }), fields({ title: 'B' })]);
    const form = { ...initialForm(s), genre: '' };
    const change = changeFrom(s, form, ALL);
    expect(change.genre).toBe('');
    expect(change.title).toBeNull();
    expect(changeFrom(s, { ...form, title: 'Same' }, ALL).title).toBe('Same');
  });

  it('only writes the fields on offer', () => {
    const s = shared([fields({})]);
    const form = { ...initialForm(s), title: 'X', artist: 'Y' };
    const change = changeFrom(s, form, ['artist']);
    expect(change.title).toBeNull();
    expect(change.artist).toBe('Y');
  });

  it('checks numbers', () => {
    expect(invalid('year', '1999')).toBeNull();
    expect(invalid('year', '')).toBeNull();
    expect(invalid('year', '19x9')).not.toBeNull();
    expect(invalid('year', '20000')).not.toBeNull();
    expect(invalid('trackNo', '0')).not.toBeNull();
    expect(invalid('title', '12abc')).toBeNull();
  });
});
