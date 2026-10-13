//! SQLite storage: schema, writes from the scanner, and every query the UI makes.

use std::collections::{BTreeMap, HashMap};
use std::path::{MAIN_SEPARATOR, Path};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::artists::norm;
use crate::model::{
    AlbumDetail, AlbumRow, AlbumSort, ArtistDetail, ArtistRow, FolderEntry, FolderListing,
    GenreDetail, GenreRow, LibraryFolder, LibraryOverview, SearchResults, TrackRow, TrackSort,
};
use crate::tags::ScannedTrack;

mod images;
mod user;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE folders(id INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE, added_at INTEGER NOT NULL);
CREATE TABLE albums(
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    has_album_artist INTEGER NOT NULL,
    group_key TEXT NOT NULL UNIQUE,
    year INTEGER,
    cover TEXT,
    added_at INTEGER NOT NULL
);
CREATE TABLE artists(id INTEGER PRIMARY KEY, name TEXT NOT NULL, norm TEXT NOT NULL UNIQUE);
CREATE TABLE genres(id INTEGER PRIMARY KEY, name TEXT NOT NULL, norm TEXT NOT NULL UNIQUE);
CREATE TABLE tracks(
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    folder_id INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    dir TEXT NOT NULL,
    mtime INTEGER NOT NULL,
    size INTEGER NOT NULL,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album_id INTEGER NOT NULL REFERENCES albums(id),
    track_no INTEGER,
    disc_no INTEGER,
    year INTEGER,
    duration_ms INTEGER NOT NULL,
    codec TEXT NOT NULL,
    sample_rate INTEGER NOT NULL,
    bits INTEGER NOT NULL,
    channels INTEGER NOT NULL,
    lossless INTEGER NOT NULL,
    dsd INTEGER NOT NULL,
    cover TEXT,
    added_at INTEGER NOT NULL
);
CREATE INDEX tracks_album ON tracks(album_id, disc_no, track_no);
CREATE INDEX tracks_dir ON tracks(dir);
CREATE INDEX tracks_folder ON tracks(folder_id);
CREATE INDEX tracks_added ON tracks(added_at);
CREATE TABLE track_artists(
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    artist_id INTEGER NOT NULL REFERENCES artists(id),
    role INTEGER NOT NULL,
    position INTEGER NOT NULL,
    PRIMARY KEY(track_id, artist_id, role)
);
CREATE INDEX track_artists_artist ON track_artists(artist_id);
CREATE TABLE track_genres(
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    genre_id INTEGER NOT NULL REFERENCES genres(id),
    PRIMARY KEY(track_id, genre_id)
);
CREATE INDEX track_genres_genre ON track_genres(genre_id);
CREATE VIRTUAL TABLE tracks_fts USING fts5(title, artists, album, genres,
    tokenize = 'unicode61 remove_diacritics 2', prefix = '1 2 3');
CREATE VIRTUAL TABLE albums_fts USING fts5(title, artist,
    tokenize = 'unicode61 remove_diacritics 2', prefix = '1 2 3');
CREATE VIRTUAL TABLE artists_fts USING fts5(name,
    tokenize = 'unicode61 remove_diacritics 2', prefix = '1 2 3');
"#;

/// Track artist vs. album artist in `track_artists.role`.
const ROLE_ARTIST: i64 = 0;
const ROLE_ALBUM_ARTIST: i64 = 1;

const TRACK_COLUMNS: &str = "t.id, t.title, t.artist, a.title, t.album_id, t.track_no, t.disc_no, \
    t.year, t.duration_ms, t.codec, t.sample_rate, t.bits, t.lossless, t.dsd, \
    COALESCE(t.cover, a.cover), t.path, EXISTS(SELECT 1 FROM favorites fv WHERE fv.path = t.path)";

const ALBUM_SELECT: &str = "SELECT a.id, a.title, a.artist, a.year, COUNT(t.id), \
    COALESCE(SUM(t.duration_ms), 0), a.cover, a.added_at \
    FROM albums a JOIN tracks t ON t.album_id = a.id";

/// An artist's picture: one found online, else the cover of one of their albums.
const ARTIST_SELECT: &str = "SELECT ar.id, ar.name, COUNT(DISTINCT t.album_id), COUNT(DISTINCT t.id), \
    COALESCE((SELECT ai.image FROM artist_images ai WHERE ai.norm = ar.norm), \
    (SELECT COALESCE(t2.cover, a2.cover) FROM track_artists ta2 \
        JOIN tracks t2 ON t2.id = ta2.track_id JOIN albums a2 ON a2.id = t2.album_id \
        WHERE ta2.artist_id = ar.id AND COALESCE(t2.cover, a2.cover) IS NOT NULL LIMIT 1)) \
    FROM artists ar JOIN track_artists ta ON ta.artist_id = ar.id JOIN tracks t ON t.id = ta.track_id";

const GENRE_SELECT: &str = "SELECT g.id, g.name, COUNT(tg.track_id), \
    (SELECT COALESCE(t2.cover, a2.cover) FROM track_genres tg2 \
        JOIN tracks t2 ON t2.id = tg2.track_id JOIN albums a2 ON a2.id = t2.album_id \
        WHERE tg2.genre_id = g.id AND COALESCE(t2.cover, a2.cover) IS NOT NULL LIMIT 1) \
    FROM genres g JOIN track_genres tg ON tg.genre_id = g.id";

/// A file found on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileStat {
    pub path: String,
    pub dir: String,
    /// Milliseconds since the Unix epoch.
    pub mtime: i64,
    pub size: i64,
}

/// A file read by the scanner, ready to be written.
#[derive(Clone, Debug)]
pub struct ScannedFile {
    pub stat: FileStat,
    pub track: ScannedTrack,
    pub cover: Option<String>,
}

fn track_row(r: &Row) -> rusqlite::Result<TrackRow> {
    Ok(TrackRow {
        id: r.get(0)?,
        title: r.get(1)?,
        artist: r.get(2)?,
        album: r.get(3)?,
        album_id: r.get(4)?,
        track_no: r.get(5)?,
        disc_no: r.get(6)?,
        year: r.get(7)?,
        duration_ms: r.get::<_, i64>(8)?.clamp(0, i64::from(u32::MAX)) as u32,
        codec: r.get(9)?,
        sample_rate: r.get(10)?,
        bits: r.get(11)?,
        lossless: r.get(12)?,
        dsd: r.get(13)?,
        cover: r.get(14)?,
        path: r.get(15)?,
        favorite: r.get(16)?,
    })
}

fn album_row(r: &Row) -> rusqlite::Result<AlbumRow> {
    Ok(AlbumRow {
        id: r.get(0)?,
        title: r.get(1)?,
        artist: r.get(2)?,
        year: r.get(3)?,
        track_count: r.get(4)?,
        duration_ms: r.get::<_, i64>(5)? as f64,
        cover: r.get(6)?,
        added_at: r.get::<_, i64>(7)?.clamp(0, i64::from(u32::MAX)) as u32,
    })
}

fn artist_row(r: &Row) -> rusqlite::Result<ArtistRow> {
    Ok(ArtistRow {
        id: r.get(0)?,
        name: r.get(1)?,
        album_count: r.get(2)?,
        track_count: r.get(3)?,
        cover: r.get(4)?,
    })
}

fn genre_row(r: &Row) -> rusqlite::Result<GenreRow> {
    Ok(GenreRow {
        id: r.get(0)?,
        name: r.get(1)?,
        track_count: r.get(2)?,
        cover: r.get(3)?,
    })
}

/// Album grouping: (title, album artist) when the album artist is tagged, otherwise
/// (title, folder) so compilations without an album artist stay together.
fn album_identity(t: &ScannedTrack, dir: &str) -> (String, String, String, bool) {
    let title = t.album.clone().unwrap_or_default();
    match (&t.album_artist, title.is_empty()) {
        (Some(aa), false) => (
            title.clone(),
            format!("{}\u{1f}{}", norm(&title), norm(aa)),
            aa.clone(),
            true,
        ),
        _ => (
            title.clone(),
            format!("{}\u{1f}dir\u{1f}{}", norm(&title), dir.to_lowercase()),
            t.artist.clone(),
            false,
        ),
    }
}

fn is_various(name: &str) -> bool {
    matches!(
        norm(name).as_str(),
        "various artists" | "various" | "va" | "v.a."
    )
}

/// FTS5 query matching every word as a prefix: `abba wat` → `"abba"* "wat"*`.
pub fn fts_query(q: &str) -> Option<String> {
    let terms: Vec<String> = q
        .split_whitespace()
        .map(|w| w.replace('"', " "))
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{w}\"*"))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

fn like_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// ORDER BY clause for a track sort.
fn track_order(sort: TrackSort) -> &'static str {
    match sort {
        TrackSort::Title => "t.title COLLATE NOCASE, t.id",
        TrackSort::Artist => {
            "t.artist COLLATE NOCASE, a.title COLLATE NOCASE, COALESCE(t.disc_no, 1), t.track_no, t.id"
        }
        TrackSort::Album => {
            "a.title COLLATE NOCASE, a.id, COALESCE(t.disc_no, 1), t.track_no, t.id"
        }
        TrackSort::Year => {
            "t.year DESC NULLS LAST, a.title COLLATE NOCASE, COALESCE(t.disc_no, 1), t.track_no"
        }
        TrackSort::Added => "t.added_at DESC, t.id DESC",
        TrackSort::Duration => "t.duration_ms DESC, t.id",
    }
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(10))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;",
        )?;
        let mut db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&mut self) -> rusqlite::Result<()> {
        let version: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 1 {
            let tx = self.conn.transaction()?;
            tx.execute_batch(SCHEMA)?;
            tx.execute_batch("PRAGMA user_version = 1")?;
            tx.commit()?;
        }
        if version < 2 {
            let tx = self.conn.transaction()?;
            tx.execute_batch(user::SCHEMA_V2)?;
            tx.execute_batch("PRAGMA user_version = 2")?;
            tx.commit()?;
        }
        if version < 3 {
            let tx = self.conn.transaction()?;
            tx.execute_batch(images::SCHEMA_V3)?;
            tx.execute_batch("PRAGMA user_version = 3")?;
            tx.commit()?;
        }
        Ok(())
    }

    // ── Settings ────────────────────────────────────────────────────────────────

    pub fn setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO meta(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ── Folders ─────────────────────────────────────────────────────────────────

    pub fn folders(&self) -> rusqlite::Result<Vec<LibraryFolder>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT f.id, f.path, (SELECT COUNT(*) FROM tracks t WHERE t.folder_id = f.id) \
             FROM folders f ORDER BY f.path COLLATE NOCASE",
        )?;
        stmt.query_map([], |r| {
            Ok(LibraryFolder {
                id: r.get(0)?,
                path: r.get(1)?,
                track_count: r.get(2)?,
            })
        })?
        .collect()
    }

    pub fn add_folder(&self, path: &str, now: i64) -> rusqlite::Result<u32> {
        self.conn.query_row(
            "INSERT INTO folders(path, added_at) VALUES(?1, ?2) \
             ON CONFLICT(path) DO UPDATE SET path = excluded.path RETURNING id",
            params![path, now],
            |r| r.get(0),
        )
    }

    pub fn remove_folder(&mut self, id: u32) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM tracks_fts WHERE rowid IN (SELECT id FROM tracks WHERE folder_id = ?1)",
            [id],
        )?;
        tx.execute("DELETE FROM folders WHERE id = ?1", [id])?;
        tx.commit()?;
        self.finish_scan()
    }

    // ── Scanner writes ──────────────────────────────────────────────────────────

    /// Path → (track id, mtime, size) of the files already known in a folder.
    pub fn known_files(
        &self,
        folder_id: u32,
    ) -> rusqlite::Result<HashMap<String, (u32, i64, i64)>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT path, id, mtime, size FROM tracks WHERE folder_id = ?1")?;
        let rows = stmt.query_map([folder_id], |r| {
            Ok((r.get(0)?, (r.get(1)?, r.get(2)?, r.get(3)?)))
        })?;
        rows.collect()
    }

    /// Inserts or updates scanned files in one transaction. Returns (added, updated).
    pub fn write_tracks(
        &mut self,
        folder_id: u32,
        files: &[ScannedFile],
        now: i64,
    ) -> rusqlite::Result<(u32, u32)> {
        let (mut added, mut updated) = (0, 0);
        let tx = self.conn.transaction()?;
        {
            let mut exists = tx.prepare_cached("SELECT 1 FROM tracks WHERE path = ?1")?;
            let mut album = tx.prepare_cached(
                "INSERT INTO albums(title, artist, has_album_artist, group_key, added_at) \
                 VALUES(?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(group_key) DO UPDATE SET group_key = excluded.group_key RETURNING id",
            )?;
            let mut track = tx.prepare_cached(
                "INSERT INTO tracks(path, folder_id, dir, mtime, size, title, artist, album_id, \
                     track_no, disc_no, year, duration_ms, codec, sample_rate, bits, channels, \
                     lossless, dsd, cover, added_at) \
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20) \
                 ON CONFLICT(path) DO UPDATE SET folder_id = excluded.folder_id, dir = excluded.dir, \
                     mtime = excluded.mtime, size = excluded.size, title = excluded.title, \
                     artist = excluded.artist, album_id = excluded.album_id, track_no = excluded.track_no, \
                     disc_no = excluded.disc_no, year = excluded.year, duration_ms = excluded.duration_ms, \
                     codec = excluded.codec, sample_rate = excluded.sample_rate, bits = excluded.bits, \
                     channels = excluded.channels, lossless = excluded.lossless, dsd = excluded.dsd, \
                     cover = excluded.cover \
                 RETURNING id",
            )?;
            let mut artist = tx.prepare_cached(
                "INSERT INTO artists(name, norm) VALUES(?1, ?2) \
                 ON CONFLICT(norm) DO UPDATE SET norm = excluded.norm RETURNING id",
            )?;
            let mut genre = tx.prepare_cached(
                "INSERT INTO genres(name, norm) VALUES(?1, ?2) \
                 ON CONFLICT(norm) DO UPDATE SET norm = excluded.norm RETURNING id",
            )?;
            let mut clear_artists =
                tx.prepare_cached("DELETE FROM track_artists WHERE track_id = ?1")?;
            let mut link_artist = tx.prepare_cached(
                "INSERT OR IGNORE INTO track_artists(track_id, artist_id, role, position) VALUES(?1, ?2, ?3, ?4)",
            )?;
            let mut clear_genres =
                tx.prepare_cached("DELETE FROM track_genres WHERE track_id = ?1")?;
            let mut link_genre = tx.prepare_cached(
                "INSERT OR IGNORE INTO track_genres(track_id, genre_id) VALUES(?1, ?2)",
            )?;
            let mut clear_fts = tx.prepare_cached("DELETE FROM tracks_fts WHERE rowid = ?1")?;
            let mut add_fts = tx.prepare_cached(
                "INSERT INTO tracks_fts(rowid, title, artists, album, genres) VALUES(?1, ?2, ?3, ?4, ?5)",
            )?;

            for f in files {
                let t = &f.track;
                if exists.exists([&f.stat.path])? {
                    updated += 1;
                } else {
                    added += 1;
                }
                let (album_title, key, album_artist, has_aa) = album_identity(t, &f.stat.dir);
                let album_id: i64 = album
                    .query_row(params![album_title, album_artist, has_aa, key, now], |r| {
                        r.get(0)
                    })?;
                let track_id: i64 = track.query_row(
                    params![
                        f.stat.path,
                        folder_id,
                        f.stat.dir,
                        f.stat.mtime,
                        f.stat.size,
                        t.title,
                        t.artist,
                        album_id,
                        t.track_no,
                        t.disc_no,
                        t.year,
                        i64::try_from(t.duration_ms).unwrap_or(i64::MAX),
                        t.codec,
                        t.sample_rate,
                        t.bits,
                        t.channels,
                        t.lossless,
                        t.dsd,
                        f.cover,
                        now
                    ],
                    |r| r.get(0),
                )?;

                clear_artists.execute([track_id])?;
                for (pos, name) in t.artists.iter().enumerate() {
                    let id: i64 = artist.query_row(params![name, norm(name)], |r| r.get(0))?;
                    link_artist.execute(params![track_id, id, ROLE_ARTIST, pos as i64])?;
                }
                for (pos, name) in t
                    .album_artists
                    .iter()
                    .filter(|n| !is_various(n))
                    .enumerate()
                {
                    let id: i64 = artist.query_row(params![name, norm(name)], |r| r.get(0))?;
                    link_artist.execute(params![track_id, id, ROLE_ALBUM_ARTIST, pos as i64])?;
                }
                clear_genres.execute([track_id])?;
                for name in &t.genres {
                    let id: i64 = genre.query_row(params![name, norm(name)], |r| r.get(0))?;
                    link_genre.execute(params![track_id, id])?;
                }

                clear_fts.execute([track_id])?;
                let mut names = t.artists.clone();
                names.extend(t.album_artists.iter().cloned());
                add_fts.execute(params![
                    track_id,
                    t.title,
                    names.join(" "),
                    t.album.clone().unwrap_or_default(),
                    t.genres.join(" ")
                ])?;
            }
        }
        tx.commit()?;
        Ok((added, updated))
    }

    /// Forces every file to be re-read on the next scan (after a settings change).
    pub fn mark_all_stale(&self) -> rusqlite::Result<()> {
        self.conn.execute("UPDATE tracks SET mtime = -1", [])?;
        Ok(())
    }

    pub fn remove_tracks(&mut self, ids: &[u32]) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut fts = tx.prepare_cached("DELETE FROM tracks_fts WHERE rowid = ?1")?;
            let mut track = tx.prepare_cached("DELETE FROM tracks WHERE id = ?1")?;
            for id in ids {
                fts.execute([id])?;
                track.execute([id])?;
            }
        }
        tx.commit()
    }

    /// Album artist/year/cover aggregates, orphan cleanup and the album/artist search index.
    pub fn finish_scan(&mut self) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(
            "UPDATE albums SET artist = COALESCE(
                 (SELECT CASE WHEN COUNT(DISTINCT t.artist) = 1 THEN MIN(t.artist) ELSE 'Various artists' END
                  FROM tracks t WHERE t.album_id = albums.id), artist)
             WHERE has_album_artist = 0;
             UPDATE albums SET year = (SELECT MIN(year) FROM tracks WHERE album_id = albums.id AND year IS NOT NULL);
             UPDATE albums SET cover = (SELECT cover FROM tracks WHERE album_id = albums.id AND cover IS NOT NULL
                 ORDER BY COALESCE(disc_no, 1), COALESCE(track_no, 0) LIMIT 1);
             UPDATE albums SET added_at = COALESCE((SELECT MIN(added_at) FROM tracks WHERE album_id = albums.id), added_at);
             DELETE FROM albums WHERE NOT EXISTS (SELECT 1 FROM tracks WHERE album_id = albums.id);
             DELETE FROM artists WHERE NOT EXISTS (SELECT 1 FROM track_artists WHERE artist_id = artists.id);
             DELETE FROM genres WHERE NOT EXISTS (SELECT 1 FROM track_genres WHERE genre_id = genres.id);
             DELETE FROM albums_fts;
             INSERT INTO albums_fts(rowid, title, artist) SELECT id, title, artist FROM albums;
             DELETE FROM artists_fts;
             INSERT INTO artists_fts(rowid, name) SELECT id, name FROM artists;",
        )?;
        tx.commit()
    }

    // ── Queries ─────────────────────────────────────────────────────────────────

    pub fn overview(&self, scanning: bool) -> rusqlite::Result<LibraryOverview> {
        let (track_count, album_count, artist_count, genre_count, duration_ms) =
            self.conn.query_row(
                "SELECT (SELECT COUNT(*) FROM tracks), (SELECT COUNT(*) FROM albums), \
                    (SELECT COUNT(*) FROM artists), (SELECT COUNT(*) FROM genres), \
                    (SELECT COALESCE(SUM(duration_ms), 0) FROM tracks)",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get::<_, i64>(4)? as f64,
                    ))
                },
            )?;
        Ok(LibraryOverview {
            track_count,
            album_count,
            artist_count,
            genre_count,
            duration_ms,
            folders: self.folders()?,
            recently_added: self.albums(AlbumSort::Added, 0, 12)?,
            scanning,
        })
    }

    pub fn tracks(
        &self,
        sort: TrackSort,
        offset: u32,
        limit: u32,
    ) -> rusqlite::Result<Vec<TrackRow>> {
        let order = track_order(sort);
        let sql = format!(
            "SELECT {TRACK_COLUMNS} FROM tracks t JOIN albums a ON a.id = t.album_id \
             ORDER BY {order} LIMIT ?1 OFFSET ?2"
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        stmt.query_map(params![limit, offset], track_row)?.collect()
    }

    /// Every track id in `sort` order (to queue the whole library).
    pub fn track_ids(&self, sort: TrackSort) -> rusqlite::Result<Vec<u32>> {
        let order = track_order(sort);
        let sql = format!(
            "SELECT t.id FROM tracks t JOIN albums a ON a.id = t.album_id ORDER BY {order}"
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        stmt.query_map([], |r| r.get(0))?.collect()
    }

    pub fn albums(
        &self,
        sort: AlbumSort,
        offset: u32,
        limit: u32,
    ) -> rusqlite::Result<Vec<AlbumRow>> {
        let order = match sort {
            AlbumSort::Title => "a.title COLLATE NOCASE, a.id",
            AlbumSort::Artist => "a.artist COLLATE NOCASE, a.year, a.title COLLATE NOCASE",
            AlbumSort::Year => "a.year DESC NULLS LAST, a.title COLLATE NOCASE",
            AlbumSort::Added => "a.added_at DESC, a.id DESC",
        };
        let sql = format!("{ALBUM_SELECT} GROUP BY a.id ORDER BY {order} LIMIT ?1 OFFSET ?2");
        let mut stmt = self.conn.prepare_cached(&sql)?;
        stmt.query_map(params![limit, offset], album_row)?.collect()
    }

    pub fn album(&self, id: u32) -> rusqlite::Result<Option<AlbumDetail>> {
        let sql = format!("{ALBUM_SELECT} WHERE a.id = ?1 GROUP BY a.id");
        let Some(album) = self.conn.query_row(&sql, [id], album_row).optional()? else {
            return Ok(None);
        };
        let sql = format!(
            "SELECT {TRACK_COLUMNS} FROM tracks t JOIN albums a ON a.id = t.album_id \
             WHERE t.album_id = ?1 \
             ORDER BY COALESCE(t.disc_no, 1), COALESCE(t.track_no, 9999), t.title COLLATE NOCASE"
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let tracks = stmt
            .query_map([id], track_row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Some(AlbumDetail { album, tracks }))
    }

    pub fn artists(&self, offset: u32, limit: u32) -> rusqlite::Result<Vec<ArtistRow>> {
        let sql = format!(
            "{ARTIST_SELECT} GROUP BY ar.id ORDER BY ar.name COLLATE NOCASE LIMIT ?1 OFFSET ?2"
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        stmt.query_map(params![limit, offset], artist_row)?
            .collect()
    }

    pub fn artist(&self, id: u32) -> rusqlite::Result<Option<ArtistDetail>> {
        let sql = format!("{ARTIST_SELECT} WHERE ar.id = ?1 GROUP BY ar.id");
        let Some(artist) = self.conn.query_row(&sql, [id], artist_row).optional()? else {
            return Ok(None);
        };
        let sql = format!(
            "{ALBUM_SELECT} WHERE a.id IN (SELECT t2.album_id FROM tracks t2 \
                 JOIN track_artists ta ON ta.track_id = t2.id WHERE ta.artist_id = ?1) \
             GROUP BY a.id ORDER BY a.year NULLS LAST, a.title COLLATE NOCASE"
        );
        let albums = self
            .conn
            .prepare_cached(&sql)?
            .query_map([id], album_row)?
            .collect::<rusqlite::Result<_>>()?;
        let sql = format!(
            "SELECT {TRACK_COLUMNS} FROM tracks t JOIN albums a ON a.id = t.album_id \
             WHERE t.id IN (SELECT track_id FROM track_artists WHERE artist_id = ?1 AND role = {ROLE_ARTIST}) \
             ORDER BY a.year NULLS LAST, a.title COLLATE NOCASE, COALESCE(t.disc_no, 1), t.track_no LIMIT 2000"
        );
        let tracks = self
            .conn
            .prepare_cached(&sql)?
            .query_map([id], track_row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Some(ArtistDetail {
            artist,
            albums,
            tracks,
        }))
    }

    pub fn genres(&self) -> rusqlite::Result<Vec<GenreRow>> {
        let sql = format!(
            "{GENRE_SELECT} GROUP BY g.id ORDER BY COUNT(tg.track_id) DESC, g.name COLLATE NOCASE"
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        stmt.query_map([], genre_row)?.collect()
    }

    pub fn genre(&self, id: u32) -> rusqlite::Result<Option<GenreDetail>> {
        let sql = format!("{GENRE_SELECT} WHERE g.id = ?1 GROUP BY g.id");
        let Some(genre) = self.conn.query_row(&sql, [id], genre_row).optional()? else {
            return Ok(None);
        };
        let sql = format!(
            "{ALBUM_SELECT} WHERE a.id IN (SELECT t2.album_id FROM tracks t2 \
                 JOIN track_genres tg ON tg.track_id = t2.id WHERE tg.genre_id = ?1) \
             GROUP BY a.id ORDER BY a.artist COLLATE NOCASE, a.year"
        );
        let albums = self
            .conn
            .prepare_cached(&sql)?
            .query_map([id], album_row)?
            .collect::<rusqlite::Result<_>>()?;
        let sql = format!(
            "SELECT {TRACK_COLUMNS} FROM tracks t JOIN albums a ON a.id = t.album_id \
             WHERE t.id IN (SELECT track_id FROM track_genres WHERE genre_id = ?1) \
             ORDER BY t.artist COLLATE NOCASE, a.title COLLATE NOCASE, COALESCE(t.disc_no, 1), t.track_no LIMIT 2000"
        );
        let tracks = self
            .conn
            .prepare_cached(&sql)?
            .query_map([id], track_row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Some(GenreDetail {
            genre,
            albums,
            tracks,
        }))
    }

    /// Browses by folder. `None` lists the library folders themselves.
    pub fn folder(&self, path: Option<&str>) -> rusqlite::Result<FolderListing> {
        let roots = self.folders()?;
        let Some(path) = path.map(|p| p.trim_end_matches(MAIN_SEPARATOR)) else {
            return Ok(FolderListing {
                path: None,
                name: "Folders".into(),
                parent: None,
                folders: roots
                    .iter()
                    .map(|r| FolderEntry {
                        path: r.path.clone(),
                        name: r.path.clone(),
                        track_count: r.track_count,
                    })
                    .collect(),
                tracks: Vec::new(),
            });
        };

        let sep = MAIN_SEPARATOR.to_string();
        let mut stmt = self.conn.prepare_cached(
            "SELECT dir, COUNT(*) FROM tracks WHERE dir = ?1 OR dir LIKE ?2 ESCAPE '\\' GROUP BY dir",
        )?;
        let pattern = format!("{}{}%", like_escape(path), like_escape(&sep));
        let mut children: BTreeMap<String, u32> = BTreeMap::new();
        for row in stmt.query_map(params![path, pattern], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))
        })? {
            let (dir, count) = row?;
            if dir == path {
                continue;
            }
            let rest = &dir[path.len() + sep.len()..];
            let first = rest.split(MAIN_SEPARATOR).next().unwrap_or(rest);
            *children.entry(first.to_string()).or_default() += count;
        }
        let folders = children
            .into_iter()
            .map(|(name, track_count)| FolderEntry {
                path: format!("{path}{sep}{name}"),
                name,
                track_count,
            })
            .collect();

        let sql = format!(
            "SELECT {TRACK_COLUMNS} FROM tracks t JOIN albums a ON a.id = t.album_id \
             WHERE t.dir = ?1 ORDER BY COALESCE(t.disc_no, 1), COALESCE(t.track_no, 9999), t.path"
        );
        let tracks = self
            .conn
            .prepare_cached(&sql)?
            .query_map([path], track_row)?
            .collect::<rusqlite::Result<_>>()?;

        let is_root = roots
            .iter()
            .any(|r| r.path.trim_end_matches(MAIN_SEPARATOR) == path);
        let parent = if is_root {
            None
        } else {
            Path::new(path)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
        };
        let name = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string());
        Ok(FolderListing {
            path: Some(path.to_string()),
            name,
            parent,
            folders,
            tracks,
        })
    }

    pub fn search(&self, query: &str, limit: u32) -> rusqlite::Result<SearchResults> {
        let Some(q) = fts_query(query) else {
            return Ok(SearchResults::default());
        };
        let sql = format!(
            "SELECT {TRACK_COLUMNS} FROM tracks_fts f JOIN tracks t ON t.id = f.rowid \
             JOIN albums a ON a.id = t.album_id WHERE tracks_fts MATCH ?1 ORDER BY f.rank LIMIT ?2"
        );
        let tracks = self
            .conn
            .prepare_cached(&sql)?
            .query_map(params![q, limit], track_row)?
            .collect::<rusqlite::Result<_>>()?;
        let sql = format!(
            "{ALBUM_SELECT} WHERE a.id IN (SELECT rowid FROM albums_fts WHERE albums_fts MATCH ?1 LIMIT ?2) \
             GROUP BY a.id ORDER BY a.title COLLATE NOCASE"
        );
        let albums = self
            .conn
            .prepare_cached(&sql)?
            .query_map(params![q, limit], album_row)?
            .collect::<rusqlite::Result<_>>()?;
        let sql = format!(
            "{ARTIST_SELECT} WHERE ar.id IN (SELECT rowid FROM artists_fts WHERE artists_fts MATCH ?1 LIMIT ?2) \
             GROUP BY ar.id ORDER BY ar.name COLLATE NOCASE"
        );
        let artists = self
            .conn
            .prepare_cached(&sql)?
            .query_map(params![q, limit], artist_row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(SearchResults {
            tracks,
            albums,
            artists,
        })
    }

    /// Tracks by id, in the order asked for; unknown ids are skipped.
    pub fn tracks_by_id(&self, ids: &[u32]) -> rusqlite::Result<Vec<TrackRow>> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {TRACK_COLUMNS} FROM tracks t JOIN albums a ON a.id = t.album_id WHERE t.id = ?1"
        ))?;
        let mut out = Vec::with_capacity(ids.len());
        for &id in ids {
            if let Some(row) = stmt.query_row([id], track_row).optional()? {
                out.push(row);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_queries_are_prefix_terms() {
        assert_eq!(fts_query("abba wat").as_deref(), Some("\"abba\"* \"wat\"*"));
        assert_eq!(fts_query("  \" ").as_deref(), None);
        assert_eq!(fts_query("say \"hi\"").as_deref(), Some("\"say\"* \"hi\"*"));
    }

    #[test]
    fn version_1_databases_gain_the_newer_tables() {
        let dir = std::env::temp_dir().join(format!("pp-db-migrate-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("v1.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(SCHEMA).unwrap();
            conn.execute_batch("PRAGMA user_version = 1").unwrap();
        }
        let db = Db::open(&path).unwrap();
        let version: i64 = db
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 3);
        db.set_favorite("/a.flac", true, 1).unwrap();
        assert!(db.is_favorite("/a.flac").unwrap());
        db.set_artist_image("björk", Some("abc"), 1).unwrap();
        drop(db);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn like_patterns_are_escaped() {
        assert_eq!(like_escape("a_b%c\\d"), "a\\_b\\%c\\\\d");
    }
}
