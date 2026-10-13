//! The user's own data, kept apart from what scans produce: playlists, favourites and play
//! history. Everything is keyed by file path, so rescans and removed folders never touch it.

use std::collections::HashSet;

use rusqlite::{OptionalExtension, Row, Transaction, params};

use super::{ALBUM_SELECT, ARTIST_SELECT, Db, TRACK_COLUMNS, album_row, artist_row, track_row};
use crate::model::{
    AlbumRow, DayListening, ListeningStats, PlayRecord, PlaylistDetail, PlaylistInput, PlaylistRow,
    TopAlbum, TopArtist, TopTrack, TrackRow,
};

pub(super) const SCHEMA_V2: &str = r#"
CREATE TABLE playlists(
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE playlist_items(
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    path TEXT NOT NULL,
    title TEXT,
    artist TEXT,
    duration_ms INTEGER,
    PRIMARY KEY(playlist_id, position)
);
CREATE INDEX playlist_items_path ON playlist_items(path);
CREATE TABLE favorites(path TEXT PRIMARY KEY, added_at INTEGER NOT NULL);
CREATE TABLE plays(
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    played_at INTEGER NOT NULL,
    ms_played INTEGER NOT NULL,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT NOT NULL
);
CREATE INDEX plays_time ON plays(played_at);
CREATE INDEX plays_path ON plays(path, played_at);
"#;

const PLAYLIST_SELECT: &str = "SELECT p.id, p.name, COUNT(i.position), \
    COALESCE(SUM(COALESCE(t.duration_ms, i.duration_ms, 0)), 0), p.updated_at \
    FROM playlists p LEFT JOIN playlist_items i ON i.playlist_id = p.id \
    LEFT JOIN tracks t ON t.path = i.path";

const DAY: i64 = 86_400;

fn playlist_row(r: &Row) -> rusqlite::Result<PlaylistRow> {
    Ok(PlaylistRow {
        id: r.get(0)?,
        name: r.get(1)?,
        track_count: r.get(2)?,
        duration_ms: r.get::<_, i64>(3)? as f64,
        covers: Vec::new(),
        updated_at: r.get::<_, i64>(4)?.clamp(0, i64::from(u32::MAX)) as u32,
    })
}

/// Small seeded PRNG (xorshift64) for reproducible daily mixes.
struct XorShift(u64);

impl XorShift {
    fn below(&mut self, n: usize) -> usize {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x % n.max(1) as u64) as usize
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, self.below(i + 1));
        }
    }
}

fn rewrite_items(tx: &Transaction, id: u32, items: &[PlaylistInput]) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM playlist_items WHERE playlist_id = ?1", [id])?;
    let mut stmt = tx.prepare_cached(
        "INSERT INTO playlist_items(playlist_id, position, path, title, artist, duration_ms) \
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (i, item) in items.iter().enumerate() {
        stmt.execute(params![
            id,
            i as i64,
            item.path,
            item.title,
            item.artist,
            item.duration_ms
        ])?;
    }
    Ok(())
}

impl Db {
    // ── Playlists ───────────────────────────────────────────────────────────────

    pub fn playlists(&self) -> rusqlite::Result<Vec<PlaylistRow>> {
        let sql = format!("{PLAYLIST_SELECT} GROUP BY p.id ORDER BY p.updated_at DESC, p.id DESC");
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let rows = stmt
            .query_map([], playlist_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter()
            .map(|mut row| {
                row.covers = self.playlist_covers(row.id)?;
                Ok(row)
            })
            .collect()
    }

    /// The first four different covers, in playlist order.
    fn playlist_covers(&self, id: u32) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT COALESCE(t.cover, a.cover) FROM playlist_items i \
             JOIN tracks t ON t.path = i.path JOIN albums a ON a.id = t.album_id \
             WHERE i.playlist_id = ?1 AND COALESCE(t.cover, a.cover) IS NOT NULL \
             ORDER BY i.position",
        )?;
        let mut covers: Vec<String> = Vec::new();
        for cover in stmt.query_map([id], |r| r.get::<_, String>(0))? {
            let cover = cover?;
            if !covers.contains(&cover) {
                covers.push(cover);
                if covers.len() == 4 {
                    break;
                }
            }
        }
        Ok(covers)
    }

    pub fn playlist(&self, id: u32) -> rusqlite::Result<Option<PlaylistDetail>> {
        let sql = format!("{PLAYLIST_SELECT} WHERE p.id = ?1 GROUP BY p.id");
        let Some(mut playlist) = self.conn.query_row(&sql, [id], playlist_row).optional()? else {
            return Ok(None);
        };
        playlist.covers = self.playlist_covers(id)?;
        // Library tracks come back as full rows; anything else is shown from the stored hints.
        let sql = format!(
            "SELECT {TRACK_COLUMNS}, i.path, i.title, i.artist, i.duration_ms \
             FROM playlist_items i LEFT JOIN tracks t ON t.path = i.path \
             LEFT JOIN albums a ON a.id = t.album_id \
             WHERE i.playlist_id = ?1 ORDER BY i.position"
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let tracks = stmt
            .query_map([id], |r| {
                if r.get::<_, Option<u32>>(0)?.is_some() {
                    return track_row(r);
                }
                let path: String = r.get(17)?;
                Ok(TrackRow::outside(
                    &path,
                    r.get::<_, Option<String>>(18)?.as_deref(),
                    r.get::<_, Option<String>>(19)?.as_deref(),
                    r.get::<_, Option<u32>>(20)?.unwrap_or(0),
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Some(PlaylistDetail { playlist, tracks }))
    }

    /// The items of a playlist as stored (for playing and exporting).
    pub fn playlist_items(&self, id: u32) -> rusqlite::Result<Vec<PlaylistInput>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT path, title, artist, duration_ms FROM playlist_items \
             WHERE playlist_id = ?1 ORDER BY position",
        )?;
        stmt.query_map([id], |r| {
            Ok(PlaylistInput {
                path: r.get(0)?,
                title: r.get(1)?,
                artist: r.get(2)?,
                duration_ms: r.get(3)?,
            })
        })?
        .collect()
    }

    pub fn create_playlist(&self, name: &str, now: i64) -> rusqlite::Result<u32> {
        self.conn.query_row(
            "INSERT INTO playlists(name, created_at, updated_at) VALUES(?1, ?2, ?2) RETURNING id",
            params![name, now],
            |r| r.get(0),
        )
    }

    pub fn rename_playlist(&self, id: u32, name: &str, now: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE playlists SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name, now],
        )?;
        Ok(())
    }

    pub fn delete_playlist(&self, id: u32) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM playlists WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Appends items at the end.
    pub fn playlist_add(
        &mut self,
        id: u32,
        items: &[PlaylistInput],
        now: i64,
    ) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        let start: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM playlist_items WHERE playlist_id = ?1",
            [id],
            |r| r.get(0),
        )?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO playlist_items(playlist_id, position, path, title, artist, duration_ms) \
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for (i, item) in items.iter().enumerate() {
                stmt.execute(params![
                    id,
                    start + i as i64,
                    item.path,
                    item.title,
                    item.artist,
                    item.duration_ms
                ])?;
            }
        }
        tx.execute(
            "UPDATE playlists SET updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        tx.commit()
    }

    /// Edits the item list with `edit` and stores the result.
    fn edit_items(
        &mut self,
        id: u32,
        now: i64,
        edit: impl FnOnce(&mut Vec<PlaylistInput>),
    ) -> rusqlite::Result<()> {
        let mut items = self.playlist_items(id)?;
        edit(&mut items);
        let tx = self.conn.transaction()?;
        rewrite_items(&tx, id, &items)?;
        tx.execute(
            "UPDATE playlists SET updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        tx.commit()
    }

    /// Removes the items at `positions` (0-based, as listed).
    pub fn playlist_remove(
        &mut self,
        id: u32,
        positions: &[u32],
        now: i64,
    ) -> rusqlite::Result<()> {
        let gone: HashSet<u32> = positions.iter().copied().collect();
        self.edit_items(id, now, |items| {
            let mut i = 0u32;
            items.retain(|_| {
                let keep = !gone.contains(&i);
                i += 1;
                keep
            });
        })
    }

    pub fn playlist_move(&mut self, id: u32, from: u32, to: u32, now: i64) -> rusqlite::Result<()> {
        self.edit_items(id, now, |items| {
            let from = from as usize;
            if from < items.len() {
                let item = items.remove(from);
                items.insert((to as usize).min(items.len()), item);
            }
        })
    }

    // ── Favourites ──────────────────────────────────────────────────────────────

    pub fn set_favorite(&self, path: &str, on: bool, now: i64) -> rusqlite::Result<()> {
        if on {
            self.conn.execute(
                "INSERT OR IGNORE INTO favorites(path, added_at) VALUES(?1, ?2)",
                params![path, now],
            )?;
        } else {
            self.conn
                .execute("DELETE FROM favorites WHERE path = ?1", [path])?;
        }
        Ok(())
    }

    pub fn is_favorite(&self, path: &str) -> rusqlite::Result<bool> {
        self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM favorites WHERE path = ?1)",
            [path],
            |r| r.get(0),
        )
    }

    /// Favourite library tracks, most recently liked first.
    pub fn favorites(&self) -> rusqlite::Result<Vec<TrackRow>> {
        let sql = format!(
            "SELECT {TRACK_COLUMNS} FROM favorites f JOIN tracks t ON t.path = f.path \
             JOIN albums a ON a.id = t.album_id ORDER BY f.added_at DESC, t.id"
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        stmt.query_map([], track_row)?.collect()
    }

    // ── Play history ────────────────────────────────────────────────────────────

    pub fn record_play(&self, p: &PlayRecord) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO plays(path, played_at, ms_played, title, artist, album) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![p.path, p.played_at, p.ms_played, p.title, p.artist, p.album],
        )?;
        Ok(())
    }

    /// Library tracks by most recent play.
    pub fn recent_tracks(&self, limit: u32) -> rusqlite::Result<Vec<TrackRow>> {
        let sql = format!(
            "SELECT {TRACK_COLUMNS} FROM \
             (SELECT path, MAX(played_at) last FROM plays GROUP BY path ORDER BY last DESC LIMIT ?1) r \
             JOIN tracks t ON t.path = r.path JOIN albums a ON a.id = t.album_id \
             ORDER BY r.last DESC"
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        // Plays of files no longer in the library are skipped, so ask for some extra.
        let mut rows: Vec<TrackRow> = stmt
            .query_map([limit * 2], track_row)?
            .collect::<rusqlite::Result<_>>()?;
        rows.truncate(limit as usize);
        Ok(rows)
    }

    /// Albums by most recent play.
    pub fn recent_albums(&self, limit: u32) -> rusqlite::Result<Vec<AlbumRow>> {
        let sql = format!(
            "WITH r AS (SELECT t2.album_id aid, MAX(p.played_at) last FROM plays p \
                JOIN tracks t2 ON t2.path = p.path GROUP BY t2.album_id ORDER BY last DESC LIMIT ?1) \
             {ALBUM_SELECT} JOIN r ON r.aid = a.id GROUP BY a.id ORDER BY MAX(r.last) DESC"
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        stmt.query_map([limit], album_row)?.collect()
    }

    /// Listening since `since` (Unix seconds; everything when 0).
    pub fn stats(
        &self,
        since: i64,
        days: Option<u32>,
        top: u32,
    ) -> rusqlite::Result<ListeningStats> {
        let (plays, total_ms, distinct_tracks) = self.conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(ms_played), 0), COUNT(DISTINCT path) FROM plays \
             WHERE played_at >= ?1",
            [since],
            |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as f64, r.get(2)?)),
        )?;

        let top_tracks = {
            let mut stmt = self.conn.prepare_cached(
                "SELECT path, COUNT(*) c, SUM(ms_played) ms, MAX(title), MAX(artist) FROM plays \
                 WHERE played_at >= ?1 GROUP BY path ORDER BY c DESC, ms DESC LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(params![since, top], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, u32>(1)?,
                        r.get::<_, i64>(2)? as f64,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let mut out = Vec::with_capacity(rows.len());
            for (path, plays, ms, title, artist) in rows {
                let track = match self.track_by_path(&path)? {
                    Some(t) => t,
                    None => TrackRow::outside(&path, Some(&title), Some(&artist), 0),
                };
                out.push(TopTrack { track, plays, ms });
            }
            out
        };

        let top_artists = {
            let mut stmt = self.conn.prepare_cached(
                "SELECT ta.artist_id, COUNT(*) c, SUM(p.ms_played) ms FROM plays p \
                 JOIN tracks t ON t.path = p.path \
                 JOIN track_artists ta ON ta.track_id = t.id AND ta.role = 0 \
                 WHERE p.played_at >= ?1 GROUP BY ta.artist_id ORDER BY c DESC, ms DESC LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(params![since, top], |r| {
                    Ok((
                        r.get::<_, u32>(0)?,
                        r.get::<_, u32>(1)?,
                        r.get::<_, i64>(2)? as f64,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let sql = format!("{ARTIST_SELECT} WHERE ar.id = ?1 GROUP BY ar.id");
            let mut artist = self.conn.prepare_cached(&sql)?;
            let mut out = Vec::with_capacity(rows.len());
            for (id, plays, ms) in rows {
                if let Some(artist) = artist.query_row([id], artist_row).optional()? {
                    out.push(TopArtist { artist, plays, ms });
                }
            }
            out
        };

        let top_albums = {
            let mut stmt = self.conn.prepare_cached(
                "SELECT t.album_id, COUNT(*) c, SUM(p.ms_played) ms FROM plays p \
                 JOIN tracks t ON t.path = p.path \
                 WHERE p.played_at >= ?1 GROUP BY t.album_id ORDER BY c DESC, ms DESC LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(params![since, top], |r| {
                    Ok((
                        r.get::<_, u32>(0)?,
                        r.get::<_, u32>(1)?,
                        r.get::<_, i64>(2)? as f64,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let sql = format!("{ALBUM_SELECT} WHERE a.id = ?1 GROUP BY a.id");
            let mut album = self.conn.prepare_cached(&sql)?;
            let mut out = Vec::with_capacity(rows.len());
            for (id, plays, ms) in rows {
                if let Some(album) = album.query_row([id], album_row).optional()? {
                    out.push(TopAlbum { album, plays, ms });
                }
            }
            out
        };

        let per_day = {
            let mut stmt = self.conn.prepare_cached(
                "SELECT date(played_at, 'unixepoch', 'localtime') d, SUM(ms_played) FROM plays \
                 WHERE played_at >= ?1 GROUP BY d ORDER BY d",
            )?;
            stmt.query_map([since], |r| {
                Ok(DayListening {
                    day: r.get(0)?,
                    ms: r.get::<_, i64>(1)? as f64,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };

        let mut per_hour = vec![0.0; 24];
        {
            let mut stmt = self.conn.prepare_cached(
                "SELECT CAST(strftime('%H', played_at, 'unixepoch', 'localtime') AS INTEGER) h, \
                 SUM(ms_played) FROM plays WHERE played_at >= ?1 GROUP BY h",
            )?;
            for row in stmt.query_map([since], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)? as f64))
            })? {
                let (h, ms) = row?;
                if let Some(slot) = per_hour.get_mut(h.clamp(0, 23) as usize) {
                    *slot = ms;
                }
            }
        }

        Ok(ListeningStats {
            days,
            total_ms,
            plays,
            distinct_tracks,
            top_tracks,
            top_artists,
            top_albums,
            per_day,
            per_hour,
        })
    }

    fn track_by_path(&self, path: &str) -> rusqlite::Result<Option<TrackRow>> {
        let sql = format!(
            "SELECT {TRACK_COLUMNS} FROM tracks t JOIN albums a ON a.id = t.album_id WHERE t.path = ?1"
        );
        self.conn.query_row(&sql, [path], track_row).optional()
    }

    /// Local date as a number (20261012), for seeding the day's mix.
    pub fn local_day(&self) -> rusqlite::Result<u64> {
        self.conn.query_row(
            "SELECT CAST(strftime('%Y%m%d', 'now', 'localtime') AS INTEGER)",
            [],
            |r| r.get::<_, i64>(0).map(|d| d.max(1) as u64),
        )
    }

    /// The day's mix: about half from the artists played most lately (songs not heard in
    /// the last two days), a quarter favourites not heard this week, the rest anything from
    /// the library. The same `seed` gives the same mix.
    pub fn daily_mix(&self, seed: u64, now: i64, limit: usize) -> rusqlite::Result<Vec<TrackRow>> {
        let mut rng = XorShift(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let ids = |sql: &str, args: &[&dyn rusqlite::ToSql]| -> rusqlite::Result<Vec<u32>> {
            let mut stmt = self.conn.prepare_cached(sql)?;
            stmt.query_map(args, |r| r.get(0))?.collect()
        };

        let mut artists = ids(
            "SELECT ta.artist_id FROM plays p JOIN tracks t ON t.path = p.path \
             JOIN track_artists ta ON ta.track_id = t.id AND ta.role = 0 \
             WHERE p.played_at >= ?1 GROUP BY ta.artist_id ORDER BY COUNT(*) DESC LIMIT 8",
            &[&(now - 60 * DAY)],
        )?;
        artists.sort_unstable();
        let mut from_artists = if artists.is_empty() {
            Vec::new()
        } else {
            let list = artists
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            ids(
                &format!(
                    "SELECT DISTINCT t.id FROM track_artists ta JOIN tracks t ON t.id = ta.track_id \
                     WHERE ta.role = 0 AND ta.artist_id IN ({list}) AND NOT EXISTS \
                     (SELECT 1 FROM plays p WHERE p.path = t.path AND p.played_at >= ?1) ORDER BY t.id"
                ),
                &[&(now - 2 * DAY)],
            )?
        };
        let mut favorites = ids(
            "SELECT t.id FROM favorites f JOIN tracks t ON t.path = f.path WHERE NOT EXISTS \
             (SELECT 1 FROM plays p WHERE p.path = t.path AND p.played_at >= ?1) ORDER BY t.id",
            &[&(now - 7 * DAY)],
        )?;
        let mut everything = ids("SELECT id FROM tracks ORDER BY id", &[])?;
        rng.shuffle(&mut from_artists);
        rng.shuffle(&mut favorites);
        rng.shuffle(&mut everything);

        let mut chosen: Vec<u32> = Vec::with_capacity(limit);
        let mut seen = HashSet::new();
        // Up to `n` new tracks from `pool`.
        let mut take = |pool: &[u32], n: usize, chosen: &mut Vec<u32>| {
            let mut taken = 0;
            for &id in pool {
                if taken >= n || chosen.len() >= limit {
                    break;
                }
                if seen.insert(id) {
                    chosen.push(id);
                    taken += 1;
                }
            }
        };
        take(&from_artists, limit / 2, &mut chosen);
        take(&favorites, limit / 4, &mut chosen);
        take(&everything, limit, &mut chosen);
        chosen.truncate(limit);
        rng.shuffle(&mut chosen);

        let mut rows = self.tracks_by_id(&chosen)?;
        spread_artists(&mut rows);
        Ok(rows)
    }
}

/// Reorders so the same artist rarely plays twice in a row.
fn spread_artists(rows: &mut [TrackRow]) {
    for i in 1..rows.len() {
        if rows[i].artist == rows[i - 1].artist
            && let Some(j) = (i + 1..rows.len()).find(|&j| rows[j].artist != rows[i - 1].artist)
        {
            rows.swap(i, j);
        }
    }
}
