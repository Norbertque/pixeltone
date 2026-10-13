/** The tag editor's form logic: values shared by the selected songs, and what to write. */
import type { TagFields } from './api';

export type FieldKey = keyof TagFields;

export interface FieldSpec {
  key: FieldKey;
  label: string;
  /** Digits only (year, track, disc). */
  numeric?: boolean;
  /** Only offered when one song is edited: the same title for ten songs is a mistake. */
  single?: boolean;
}

export const FIELDS: readonly FieldSpec[] = [
  { key: 'title', label: 'Title', single: true },
  { key: 'artist', label: 'Artist' },
  { key: 'album', label: 'Album' },
  { key: 'albumArtist', label: 'Album artist' },
  { key: 'genre', label: 'Genre' },
  { key: 'year', label: 'Year', numeric: true },
  { key: 'trackNo', label: 'Track', numeric: true, single: true },
  { key: 'discNo', label: 'Disc', numeric: true },
];

export type FormValues = Record<FieldKey, string>;
/** Per field: the value every song has, or null where they differ. */
export type Shared = Record<FieldKey, string | null>;

const KEYS = FIELDS.map((f) => f.key);

export function shared(all: readonly TagFields[]): Shared {
  const out = {} as Shared;
  for (const key of KEYS) {
    const values = new Set(all.map((t) => (t[key] ?? '').trim()));
    out[key] = values.size === 1 ? [...values][0]! : null;
  }
  return out;
}

export function initialForm(s: Shared): FormValues {
  const out = {} as FormValues;
  for (const key of KEYS) out[key] = s[key] ?? '';
  return out;
}

/**
 * The change to write: shared fields whose value changed, and fields that differed between
 * the songs once something was typed into them. Everything else stays as it is (null).
 */
export function changeFrom(s: Shared, form: FormValues, offered: readonly FieldKey[]): TagFields {
  const out: TagFields = {
    title: null,
    artist: null,
    album: null,
    albumArtist: null,
    genre: null,
    year: null,
    trackNo: null,
    discNo: null,
  };
  for (const key of offered) {
    const value = form[key].trim();
    const before = s[key];
    if (before === null ? value !== '' : value !== before) out[key] = value;
  }
  return out;
}

export function hasChanges(change: TagFields): boolean {
  return KEYS.some((k) => change[k] !== null);
}

/** A message for a value that can't be saved, or null. */
export function invalid(key: FieldKey, value: string): string | null {
  const v = value.trim();
  if (!v || !FIELDS.find((f) => f.key === key)?.numeric) return null;
  if (!/^\d+$/.test(v) || Number(v) === 0) return 'A whole number above 0';
  if (key === 'year' && Number(v) > 9999) return 'A year like 1999';
  return null;
}
