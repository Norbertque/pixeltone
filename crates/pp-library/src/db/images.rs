//! Artist pictures found online (schema v3), keyed by the artist's normalised name so they
//! survive rescans. `image` is an art store hash; NULL records a lookup that found nothing.

use rusqlite::params;

use super::Db;

pub(super) const SCHEMA_V3: &str = r#"
CREATE TABLE artist_images(
    norm TEXT PRIMARY KEY,
    image TEXT,
    checked_at INTEGER NOT NULL
);
"#;

/// Pictures found by name alone could show someone else of the same name: look them all up
/// again, checked against the artist's own titles.
pub(super) const SCHEMA_V4: &str = "DELETE FROM artist_images;";

impl Db {
    /// (name, normalised name) of artists never looked up, or whose last lookup found
    /// nothing before `retry_before` (seconds); the ones with the most tracks first.
    pub fn artists_without_images(
        &self,
        retry_before: i64,
        limit: u32,
    ) -> rusqlite::Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT ar.name, ar.norm FROM artists ar \
             JOIN track_artists ta ON ta.artist_id = ar.id \
             LEFT JOIN artist_images ai ON ai.norm = ar.norm \
             WHERE ai.norm IS NULL OR (ai.image IS NULL AND ai.checked_at < ?1) \
             GROUP BY ar.id ORDER BY COUNT(DISTINCT ta.track_id) DESC, ar.name LIMIT ?2",
        )?;
        stmt.query_map(params![retry_before, limit], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect()
    }

    pub fn set_artist_image(
        &self,
        norm: &str,
        image: Option<&str>,
        now: i64,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO artist_images(norm, image, checked_at) VALUES(?1, ?2, ?3) \
             ON CONFLICT(norm) DO UPDATE SET image = excluded.image, checked_at = excluded.checked_at",
            params![norm, image, now],
        )?;
        Ok(())
    }

    /// Up to `n` album titles (their own albums before compilations, larger first) and
    /// `n` song titles by the artist `norm`: what an online lookup checks a match against.
    pub fn artist_titles(
        &self,
        norm: &str,
        n: u32,
    ) -> rusqlite::Result<(Vec<String>, Vec<String>)> {
        let mut albums = self.conn.prepare_cached(
            "SELECT a.title FROM albums a \
             JOIN tracks t ON t.album_id = a.id \
             JOIN track_artists ta ON ta.track_id = t.id \
             JOIN artists ar ON ar.id = ta.artist_id \
             WHERE ar.norm = ?1 AND a.title <> '' \
             GROUP BY a.id \
             ORDER BY lower(a.artist) = ar.norm DESC, COUNT(DISTINCT t.id) DESC, a.title \
             LIMIT ?2",
        )?;
        let albums = albums
            .query_map(params![norm, n], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        let mut tracks = self.conn.prepare_cached(
            "SELECT t.title FROM tracks t \
             JOIN track_artists ta ON ta.track_id = t.id AND ta.role = 0 \
             JOIN artists ar ON ar.id = ta.artist_id \
             WHERE ar.norm = ?1 AND t.title <> '' \
             GROUP BY lower(t.title) ORDER BY MIN(t.id) LIMIT ?2",
        )?;
        let tracks = tracks
            .query_map(params![norm, n], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        Ok((albums, tracks))
    }

    pub fn forget_artist_images(&self) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM artist_images", [])?;
        Ok(())
    }
}
