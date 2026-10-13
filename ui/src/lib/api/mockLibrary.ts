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
  DayListening,
  FolderListing,
  GenreDetail,
  GenreRow,
  HomeFeed,
  LibraryFolder,
  LibraryOverview,
  ListeningStats,
  Lyrics,
  PlaylistDetail,
  PlaylistRow,
  SearchResults,
  TopAlbum,
  TopArtist,
  TopTrack,
  TagFields,
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
      favorite: false,
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

// ── User data: favourites, playlists, play history, lyrics ──────────────────────────

const DAY_MS = 86_400_000;
const trackByPath = new Map(tracks.map((t) => [t.path, t]));
const favoriteOrder: string[] = [];

interface MockPlaylist {
  id: number;
  name: string;
  paths: string[];
  updatedAt: number;
}
let lastPlaylistId = 0;
const playlists: MockPlaylist[] = [];
{
  const h = rng(11);
  const pickPaths = (n: number) => Array.from({ length: n }, () => tracks[Math.floor(h() * tracks.length)]!.path);
  for (const [name, n] of [['Late night', 24], ['Running', 40], ['Sunday morning', 12]] as const) {
    playlists.push({ id: ++lastPlaylistId, name, paths: pickPaths(n), updatedAt: 1_760_000_000 + lastPlaylistId * 9000 });
  }
}

interface Play {
  path: string;
  at: number;
  ms: number;
}

/** A year of synthetic plays, mostly from a handful of albums, more in the evenings. */
const plays: Play[] = (() => {
  const h = rng(7);
  const loved = [1, 4, 9, 13, 21, 34].map((a) => tracks.filter((t) => t.albumId === a));
  const out: Play[] = [];
  const now = Date.now();
  for (let d = 365; d >= 0; d--) {
    const count = Math.floor(h() * h() * 26);
    for (let i = 0; i < count; i++) {
      const pool = h() < 0.6 ? loved[Math.floor(h() * loved.length)]! : tracks;
      const t = pool[Math.floor(h() * pool.length)]!;
      const hour = Math.floor(8 + h() * h() * 16);
      const at = now - d * DAY_MS - (new Date(now).getHours() - hour) * 3_600_000;
      if (at <= now) out.push({ path: t.path, at, ms: Math.round(t.durationMs * (0.55 + h() * 0.45)) });
    }
  }
  return out.sort((a, b) => a.at - b.at);
})();

function localDay(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

function playlistRow(p: MockPlaylist): PlaylistRow {
  const rows = p.paths.map((path) => trackByPath.get(path)).filter((t): t is TrackRow => !!t);
  const covers = [...new Set(rows.map((t) => t.cover).filter((c): c is string => !!c))].slice(0, 4);
  return {
    id: p.id,
    name: p.name,
    trackCount: p.paths.length,
    durationMs: rows.reduce((s, t) => s + t.durationMs, 0),
    covers,
    updatedAt: p.updatedAt,
  };
}

function top<K>(items: readonly Play[], key: (p: Play) => K | undefined, n: number): { key: K; plays: number; ms: number }[] {
  const acc = new Map<K, { plays: number; ms: number }>();
  for (const p of items) {
    const k = key(p);
    if (k === undefined) continue;
    const e = acc.get(k) ?? { plays: 0, ms: 0 };
    e.plays += 1;
    e.ms += p.ms;
    acc.set(k, e);
  }
  return [...acc]
    .sort(([, a], [, b]) => b.plays - a.plays || b.ms - a.ms)
    .slice(0, n)
    .map(([key, e]) => ({ key, ...e }));
}

const albumArtist = (albumId: number) => albums.find((a) => a.row.id === albumId)?.artistId;

const DEMO_LRC = [
  '[00:00.00]',
  '[00:12.40]Salt on the glass, the harbour lights hum',
  '[00:18.90]Every wave a pixel, every pixel a drum',
  '[00:25.30]We drift where the cables go under the sea',
  '[00:31.80]And the tide keeps the time for you and me',
  '[00:38.20]',
  '[00:44.60]Neon tidepool, glowing in the deep',
  '[00:50.90]Holding all the colours that the city couldn\'t keep',
  '[00:57.40]Neon tidepool, slow and bright and blue',
  '[01:03.80]I would light the whole coastline for you',
  '[01:10.20]',
  '[01:22.00]Static in the shallows, a signal from the shore',
  '[01:28.50]Counting every echo till it isn\'t there no more',
].join('\n');

const savedLyrics = new Map<string, string>();

function lyricsFromText(text: string, source: Lyrics['source']): Lyrics {
  const lines = text.split('\n').map((raw) => {
    const m = /^\[(\d+):(\d+)(?:\.(\d+))?\](.*)$/.exec(raw.trim());
    if (!m) return { timeMs: null, text: raw.trim() };
    const frac = (m[3] ?? '0').padEnd(3, '0').slice(0, 3);
    return { timeMs: (Number(m[1]) * 60 + Number(m[2])) * 1000 + Number(frac), text: m[4]!.trim() };
  });
  const synced = lines.some((l) => l.timeMs !== null);
  return { source, synced, instrumental: false, lines: synced ? lines.filter((l) => l.timeMs !== null) : lines, text };
}

/** The mock's tag editor: reads from and writes into the generated rows. */
export const mockTags = {
  read(path: string): TagFields {
    const t = trackByPath.get(path);
    return {
      title: t?.title ?? '',
      artist: t?.artist ?? '',
      album: t?.album ?? '',
      albumArtist: '',
      genre: '',
      year: t?.year ? String(t.year) : '',
      trackNo: t?.trackNo ? String(t.trackNo) : '',
      discNo: t?.discNo ? String(t.discNo) : '',
    };
  },
  write(path: string, change: TagFields): boolean {
    const t = trackByPath.get(path);
    if (!t) return false;
    const num = (v: string) => (Number.parseInt(v, 10) > 0 ? Number.parseInt(v, 10) : null);
    if (change.title !== null) t.title = change.title || path.split(/[\\/]/).pop()!;
    if (change.artist !== null) t.artist = change.artist;
    if (change.album !== null) t.album = change.album;
    if (change.year !== null) t.year = num(change.year);
    if (change.trackNo !== null) t.trackNo = num(change.trackNo);
    if (change.discNo !== null) t.discNo = num(change.discNo);
    return true;
  },
};

export const mockUserData = {
  isFavorite(path: string): boolean {
    return favoriteOrder.includes(path);
  },
  setFavorite(path: string, on: boolean): void {
    const i = favoriteOrder.indexOf(path);
    if (on && i < 0) favoriteOrder.unshift(path);
    if (!on && i >= 0) favoriteOrder.splice(i, 1);
    const row = trackByPath.get(path);
    if (row) row.favorite = on;
  },
  favorites(): TrackRow[] {
    return favoriteOrder.map((p) => trackByPath.get(p)).filter((t): t is TrackRow => !!t);
  },

  playlists(): PlaylistRow[] {
    return [...playlists].sort((a, b) => b.updatedAt - a.updatedAt).map(playlistRow);
  },
  playlist(id: number): PlaylistDetail | null {
    const p = playlists.find((x) => x.id === id);
    if (!p) return null;
    return { playlist: playlistRow(p), tracks: p.paths.map((path) => trackByPath.get(path)).filter((t): t is TrackRow => !!t) };
  },
  createPlaylist(name: string, trackIds: number[]): number {
    const paths = trackIds.map((id) => tracks.find((t) => t.id === id)?.path).filter((p): p is string => !!p);
    playlists.push({ id: ++lastPlaylistId, name: name.trim(), paths, updatedAt: Date.now() / 1000 });
    return lastPlaylistId;
  },
  editPlaylist(id: number, edit: (p: MockPlaylist) => void): void {
    const p = playlists.find((x) => x.id === id);
    if (!p) throw new Error('the playlist no longer exists');
    edit(p);
    p.updatedAt = Date.now() / 1000;
  },
  deletePlaylist(id: number): void {
    const i = playlists.findIndex((x) => x.id === id);
    if (i >= 0) playlists.splice(i, 1);
  },
  pathsOf(trackIds: number[]): string[] {
    return trackIds.map((id) => tracks.find((t) => t.id === id)?.path).filter((p): p is string => !!p);
  },

  stats(days: number | null): ListeningStats {
    const since = days === null ? 0 : Date.now() - days * DAY_MS;
    const ps = plays.filter((p) => p.at >= since);
    const topTracks: TopTrack[] = top(ps, (p) => p.path, 20).flatMap(({ key, plays, ms }) => {
      const track = trackByPath.get(key);
      return track ? [{ track, plays, ms }] : [];
    });
    const topArtists: TopArtist[] = top(ps, (p) => albumArtist(trackByPath.get(p.path)?.albumId ?? -1), 20).map(
      ({ key, plays, ms }) => ({ artist: artistRow(key), plays, ms }),
    );
    const topAlbums: TopAlbum[] = top(ps, (p) => trackByPath.get(p.path)?.albumId, 20).flatMap(({ key, plays, ms }) => {
      const album = albums.find((a) => a.row.id === key)?.row;
      return album ? [{ album, plays, ms }] : [];
    });
    const perDay = new Map<string, number>();
    const perHour = new Array<number>(24).fill(0);
    for (const p of ps) {
      const day = localDay(p.at);
      perDay.set(day, (perDay.get(day) ?? 0) + p.ms);
      perHour[new Date(p.at).getHours()]! += p.ms;
    }
    const days_: DayListening[] = [...perDay].map(([day, ms]) => ({ day, ms }));
    return {
      days,
      totalMs: ps.reduce((s, p) => s + p.ms, 0),
      plays: ps.length,
      distinctTracks: new Set(ps.map((p) => p.path)).size,
      topTracks,
      topArtists,
      topAlbums,
      perDay: days_,
      perHour,
    };
  },

  dailyMix(): TrackRow[] {
    const h = rng(Number(localDay(Date.now()).replaceAll('-', '')));
    const favouriteArtists = new Set(this.stats(60).topArtists.slice(0, 8).map((a) => a.artist.id));
    const fromArtists = tracks.filter((t) => favouriteArtists.has(albumArtist(t.albumId) ?? -1));
    const chosen = new Set<TrackRow>();
    while (chosen.size < 20 && fromArtists.length) chosen.add(fromArtists[Math.floor(h() * fromArtists.length)]!);
    while (chosen.size < 40) chosen.add(tracks[Math.floor(h() * tracks.length)]!);
    return [...chosen].sort(() => h() - 0.5);
  },

  home(): HomeFeed {
    const recent = [...plays].reverse();
    const recentTracks = [...new Set(recent.map((p) => p.path))].slice(0, 12).flatMap((p) => trackByPath.get(p) ?? []);
    const albumIds = [...new Set(recent.map((p) => trackByPath.get(p.path)?.albumId).filter((a): a is number => a !== undefined))];
    return {
      recentTracks,
      recentAlbums: albumIds.slice(0, 12).flatMap((id) => albums.find((a) => a.row.id === id)?.row ?? []),
      topArtists: this.stats(30).topArtists.slice(0, 8),
      dailyMix: this.dailyMix(),
    };
  },

  /** Demo lyrics: synced for "Pixel Dreams" and the demo track, plain or none elsewhere. */
  lyrics(path: string, title: string): Lyrics | null {
    const saved = savedLyrics.get(path);
    if (saved !== undefined) return lyricsFromText(saved, 'file');
    if (path.includes('Pixel Dreams') || title === 'Neon Tidepool') return lyricsFromText(DEMO_LRC, 'online');
    const n = [...path].reduce((a, c) => (a * 31 + c.charCodeAt(0)) >>> 0, 7) % 4;
    if (n === 0) return null;
    if (n === 1) return { source: 'online', synced: false, instrumental: true, lines: [], text: '' };
    const h = rng(n * 1000 + path.length);
    const words = Array.from({ length: 16 }, () => `${pick(WORDS)} ${pick(WORDS).toLowerCase()} in the ${pick(WORDS).toLowerCase()}`);
    if (n === 2) return lyricsFromText(words.join('\n'), 'embedded');
    return lyricsFromText(words.map((w, i) => `[00:${String(10 + i * 4 + Math.floor(h() * 2)).padStart(2, '0')}.00]${w}`).join('\n'), 'file');
  },
  saveLyrics(path: string, text: string): Lyrics {
    if (!text.trim()) throw new Error('there are no lyrics to save');
    savedLyrics.set(path, text);
    return lyricsFromText(text, 'file');
  },
};
