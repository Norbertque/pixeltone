/**
 * A deterministic synthetic library for the browser preview: ~2,000 tracks, generated
 * cover art, and in-memory versions of every library query.
 */
import { scallopPath } from '../shapes';
import type {
  AlbumDetail,
  AlbumRow,
  AlbumSort,
  ArtistDetail,
  ArtistRow,
  FolderListing,
  GenreDetail,
  GenreRow,
  LibraryFolder,
  LibraryOverview,
  SearchResults,
  TrackRow,
  TrackSort,
} from './types';

const ROOT = 'C:\\Users\\You\\Music';

// Small seeded PRNG so the preview library is the same on every load.
function rng(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 2 ** 32;
  };
}

const SYLLABLES = ['ka', 'lu', 'mi', 'no', 'ra', 'se', 'ti', 'vo', 'zen', 'ae', 'lys', 'qua', 'dor', 'pha', 'nir', 'el'];
const WORDS = [
  'Neon', 'Tide', 'Glass', 'Garden', 'Ember', 'Pixel', 'Dreams', 'Orbit', 'Velvet', 'Harbor', 'Echo', 'Silver',
  'Midnight', 'Bloom', 'Static', 'Aurora', 'Paper', 'Moons', 'Signal', 'Drift', 'Golden', 'Hour', 'Wild', 'Tape',
  'Frost', 'Lantern', 'River', 'Static', 'Northern', 'Lights', 'Cassette', 'Haze', 'Prism', 'Sundown', 'Arcade',
];
const GENRES = ['Synthpop', 'Ambient', 'Jazz', 'Classical', 'Indie Rock', 'Electronic', 'Folk', 'Hip-Hop', 'Soul', 'Metal', 'Bossa Nova', 'Techno'];
const FORMATS = [
  { codec: 'flac', sampleRate: 44_100, bits: 16, lossless: true, dsd: false },
  { codec: 'flac', sampleRate: 96_000, bits: 24, lossless: true, dsd: false },
  { codec: 'alac', sampleRate: 192_000, bits: 24, lossless: true, dsd: false },
  { codec: 'mp3', sampleRate: 44_100, bits: 0, lossless: false, dsd: false },
  { codec: 'dsd_lsbf_planar', sampleRate: 2_822_400, bits: 0, lossless: true, dsd: true },
];

function cap(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

interface Album {
  row: AlbumRow;
  artistId: number;
  genreId: number;
  dir: string;
}

const r = rng(42);
const pick = <T>(xs: readonly T[]): T => xs[Math.floor(r() * xs.length)]!;

const artists: { id: number; name: string }[] = Array.from({ length: 60 }, (_, i) => {
  const name =
    r() < 0.4
      ? `The ${pick(WORDS)} ${pick(WORDS)}s`
      : `${cap(pick(SYLLABLES) + pick(SYLLABLES))} ${cap(pick(SYLLABLES) + pick(SYLLABLES) + pick(SYLLABLES))}`;
  return { id: i + 1, name };
});
artists[3]!.name = 'Björk';
artists[0]!.name = 'Aurora Lane';

const albums: Album[] = [];
const tracks: TrackRow[] = [];
const trackGenre = new Map<number, number>();
let trackId = 1;
for (let a = 1; a <= 170; a++) {
  const artist = artists[(a * 7) % artists.length]!;
  const genreId = (a % GENRES.length) + 1;
  const title = a === 1 ? 'Pixel Dreams' : `${pick(WORDS)} ${pick(WORDS)}`;
  const year = 1975 + Math.floor(r() * 52);
  const format = FORMATS[a % FORMATS.length]!;
  const count = 8 + Math.floor(r() * 7);
  const dir = `${ROOT}\\${artist.name}\\${title}`;
  let duration = 0;
  for (let n = 1; n <= count; n++) {
    const ms = 150_000 + Math.floor(r() * 240_000);
    duration += ms;
    const id = trackId++;
    trackGenre.set(id, genreId);
    tracks.push({
      id,
      title: n === 3 && a === 4 ? 'Jóga' : `${pick(WORDS)} ${pick(WORDS)}`,
      artist: r() < 0.08 ? `${artist.name} feat. ${pick(artists).name}` : artist.name,
      album: title,
      albumId: a,
      trackNo: n,
      discNo: null,
      year,
      durationMs: ms,
      ...format,
      cover: `m${a}`,
      path: `${dir}\\${String(n).padStart(2, '0')}.${format.codec === 'mp3' ? 'mp3' : format.dsd ? 'dsf' : 'flac'}`,
    });
  }
  albums.push({
    row: {
      id: a,
      title,
      artist: artist.name,
      year,
      trackCount: count,
      durationMs: duration,
      cover: `m${a}`,
      addedAt: 1_760_000_000 + a * 3600,
    },
    artistId: artist.id,
    genreId,
    dir,
  });
}

/** Generated cover art for mock hashes ("m<album id>"). */
export function mockArt(hash: string): string {
  const n = Number(hash.slice(1)) || 1;
  const hue = (n * 47) % 360;
  const lobes = 5 + (n % 9);
  const shape = scallopPath(600, lobes, 0.04 + (n % 5) * 0.02);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 600">
    <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="hsl(${hue} 55% 32%)"/><stop offset="1" stop-color="hsl(${(hue + 50) % 360} 70% 58%)"/></linearGradient></defs>
    <rect width="600" height="600" fill="url(#g)"/>
    <path d="${shape}" transform="translate(${120 + (n % 4) * 20} 140) scale(0.55)" fill="hsl(${(hue + 180) % 360} 80% 75%)" opacity="0.9"/>
    <circle cx="${200 + (n % 7) * 30}" cy="${420 - (n % 5) * 30}" r="${30 + (n % 6) * 8}" fill="hsl(${hue} 60% 20%)" opacity="0.8"/></svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

function fold(s: string): string {
  return s.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase();
}

function matches(text: string, query: string): boolean {
  const words = fold(text).split(/\W+/);
  return fold(query)
    .split(/\s+/)
    .filter(Boolean)
    .every((q) => words.some((w) => w.startsWith(q)) || fold(text).includes(q));
}

const byText = (a: string, b: string) => a.localeCompare(b, undefined, { sensitivity: 'base' });

function artistRow(id: number): ArtistRow {
  const own = albums.filter((a) => a.artistId === id);
  const ownTracks = tracks.filter((t) => own.some((a) => a.row.id === t.albumId));
  return {
    id,
    name: artists[id - 1]!.name,
    albumCount: own.length,
    trackCount: ownTracks.length,
    cover: own[0]?.row.cover ?? null,
  };
}

function genreRow(id: number): GenreRow {
  const ids = tracks.filter((t) => trackGenre.get(t.id) === id);
  return { id, name: GENRES[id - 1]!, trackCount: ids.length, cover: albums.find((a) => a.genreId === id)?.row.cover ?? null };
}

export const mockLibrary = {
  folders(): LibraryFolder[] {
    return [{ id: 1, path: ROOT, trackCount: tracks.length }];
  },
  overview(scanning: boolean): LibraryOverview {
    return {
      trackCount: tracks.length,
      albumCount: albums.length,
      artistCount: artists.length,
      genreCount: GENRES.length,
      durationMs: tracks.reduce((s, t) => s + t.durationMs, 0),
      folders: this.folders(),
      recentlyAdded: this.albums('added', 0, 12),
      scanning,
    };
  },
  tracks(sort: TrackSort, offset: number, limit: number): TrackRow[] {
    const sorted = [...tracks].sort((a, b) => {
      switch (sort) {
        case 'title':
          return byText(a.title, b.title);
        case 'artist':
          return byText(a.artist, b.artist) || byText(a.album, b.album) || (a.trackNo ?? 0) - (b.trackNo ?? 0);
        case 'album':
          return byText(a.album, b.album) || a.albumId - b.albumId || (a.trackNo ?? 0) - (b.trackNo ?? 0);
        case 'year':
          return (b.year ?? 0) - (a.year ?? 0) || (a.trackNo ?? 0) - (b.trackNo ?? 0);
        case 'added':
          return b.id - a.id;
        case 'duration':
          return b.durationMs - a.durationMs;
      }
    });
    return sorted.slice(offset, offset + limit);
  },
  albums(sort: AlbumSort, offset: number, limit: number): AlbumRow[] {
    const rows = albums.map((a) => a.row);
    rows.sort((a, b) => {
      switch (sort) {
        case 'title':
          return byText(a.title, b.title);
        case 'artist':
          return byText(a.artist, b.artist) || (a.year ?? 0) - (b.year ?? 0);
        case 'year':
          return (b.year ?? 0) - (a.year ?? 0);
        case 'added':
          return b.addedAt - a.addedAt;
      }
    });
    return rows.slice(offset, offset + limit);
  },
  album(id: number): AlbumDetail | null {
    const album = albums.find((a) => a.row.id === id);
    return album ? { album: album.row, tracks: tracks.filter((t) => t.albumId === id) } : null;
  },
  artists(offset: number, limit: number): ArtistRow[] {
    return [...artists]
      .sort((a, b) => byText(a.name, b.name))
      .slice(offset, offset + limit)
      .map((a) => artistRow(a.id));
  },
  artist(id: number): ArtistDetail | null {
    if (!artists[id - 1]) return null;
    const own = albums.filter((a) => a.artistId === id);
    return {
      artist: artistRow(id),
      albums: own.map((a) => a.row).sort((a, b) => (a.year ?? 0) - (b.year ?? 0)),
      tracks: tracks.filter((t) => own.some((a) => a.row.id === t.albumId)),
    };
  },
  genres(): GenreRow[] {
    return GENRES.map((_, i) => genreRow(i + 1)).sort((a, b) => b.trackCount - a.trackCount);
  },
  genre(id: number): GenreDetail | null {
    if (!GENRES[id - 1]) return null;
    return {
      genre: genreRow(id),
      albums: albums.filter((a) => a.genreId === id).map((a) => a.row),
      tracks: tracks.filter((t) => trackGenre.get(t.id) === id),
    };
  },
  folder(path: string | null): FolderListing {
    if (path === null) {
      return { path: null, name: 'Folders', parent: null, folders: [{ path: ROOT, name: ROOT, trackCount: tracks.length }], tracks: [] };
    }
    const prefix = `${path}\\`;
    const children = new Map<string, number>();
    for (const a of albums) {
      if (!a.dir.startsWith(prefix)) continue;
      const name = a.dir.slice(prefix.length).split('\\')[0]!;
      children.set(name, (children.get(name) ?? 0) + a.row.trackCount);
    }
    const here = albums.find((a) => a.dir === path);
    return {
      path,
      name: path.split('\\').pop() ?? path,
      parent: path === ROOT ? null : path.slice(0, path.lastIndexOf('\\')),
      folders: [...children].sort(([a], [b]) => byText(a, b)).map(([name, trackCount]) => ({ path: `${prefix}${name}`, name, trackCount })),
      tracks: here ? tracks.filter((t) => t.albumId === here.row.id) : [],
    };
  },
  search(query: string, limit: number): SearchResults {
    if (!query.trim()) return { tracks: [], albums: [], artists: [] };
    return {
      tracks: tracks.filter((t) => matches(`${t.title} ${t.artist} ${t.album}`, query)).slice(0, limit),
      albums: albums.filter((a) => matches(`${a.row.title} ${a.row.artist}`, query)).map((a) => a.row).slice(0, limit),
      artists: artists.filter((a) => matches(a.name, query)).slice(0, limit).map((a) => artistRow(a.id)),
    };
  },
  track(id: number): TrackRow | undefined {
    return tracks.find((t) => t.id === id);
  },
};
