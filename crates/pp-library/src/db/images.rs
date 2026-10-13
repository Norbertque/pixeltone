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

    pub fn forget_artist_images(&self) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM artist_images", [])?;
        Ok(())
    }
}
