//! Artist pictures from Deezer's public API (no account or key needed).
//!
//! A name alone isn't enough: many artists share one ("Sally"), and the best-known of them is
//! rarely the one in someone's library. So an artist is only accepted when Deezer lists an
//! album (or else a song) of theirs with a title the library has too. What's sent: the
//! artist's name with up to three album and three song titles.

use std::time::{Duration, Instant};

use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
use ureq::Agent;

pub const API: &str = "https://api.deezer.com";
/// Pictures are about 100–300 KB; anything far bigger isn't one.
const MAX_PICTURE_BYTES: u64 = 8 * 1024 * 1024;
/// Titles tried of each kind.
const TRIES: usize = 3;
/// Deezer allows 50 requests in 5 seconds; stay well below.
const SPACING: Duration = Duration::from_millis(150);

#[derive(Debug, thiserror::Error)]
pub enum DeezerError {
    #[error("{0}")]
    Http(#[from] ureq::Error),
    #[error("Deezer: {0}")]
    Api(String),
    /// Too many requests in a short time: wait a few seconds.
    #[error("Deezer asks to slow down")]
    Quota,
}

/// An artist as Deezer's search results show them.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Candidate {
    pub id: u64,
    pub name: String,
    pub picture_big: Option<String>,
    pub picture_xl: Option<String>,
}

/// An album or a song in Deezer's search results, with its artist.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Hit {
    pub title: String,
    pub artist: Candidate,
}

#[derive(Deserialize)]
struct Search {
    #[serde(default)]
    data: Vec<Hit>,
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct ApiError {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    message: String,
}

/// What the library knows the artist by, besides the name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Clues {
    /// Album titles, largest first.
    pub albums: Vec<String>,
    /// Song titles.
    pub tracks: Vec<String>,
}

/// An artist's picture on Deezer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub artist_id: u64,
    pub url: String,
}

pub struct Deezer {
    agent: Agent,
    base: String,
    last_request: parking_lot::Mutex<Option<Instant>>,
}

impl Deezer {
    pub fn new(base: &str) -> Self {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .user_agent(concat!(
                "Pixeltone/",
                env!("CARGO_PKG_VERSION"),
                " (https://github.com/Norbertque/pixeltone)"
            ))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            base: base.trim_end_matches('/').to_string(),
            last_request: parking_lot::Mutex::new(None),
        }
    }

    /// Waits so requests are at least `SPACING` apart.
    fn pace(&self) {
        let mut last = self.last_request.lock();
        if let Some(wait) = last.and_then(|t| SPACING.checked_sub(t.elapsed())) {
            std::thread::sleep(wait);
        }
        *last = Some(Instant::now());
    }

    /// The picture of the artist called `name` who made one of the `clues` titles; `None`
    /// when Deezer has no such artist, or no picture of them.
    pub fn find(&self, name: &str, clues: &Clues) -> Result<Option<Picture>, DeezerError> {
        let kinds = [("album", &clues.albums), ("track", &clues.tracks)];
        for (kind, titles) in kinds {
            for title in titles
                .iter()
                .filter(|t| !match_key(t).is_empty())
                .take(TRIES)
            {
                let hits = self.search(kind, name, title)?;
                if let Some(artist) = verified(name, title, &hits) {
                    // The right artist; a placeholder picture means Deezer has none of them.
                    return Ok(picture(artist));
                }
            }
        }
        Ok(None)
    }

    fn search(&self, kind: &str, artist: &str, title: &str) -> Result<Vec<Hit>, DeezerError> {
        let quoted = |s: &str| s.replace('"', " ");
        let query = format!(r#"artist:"{}" {kind}:"{}""#, quoted(artist), quoted(title));
        self.pace();
        let mut resp = self
            .agent
            .get(format!("{}/search/{kind}", self.base))
            .query("q", &query)
            .query("limit", "15")
            .call()?;
        if !resp.status().is_success() {
            return Err(DeezerError::Api(format!("HTTP {}", resp.status())));
        }
        let search: Search = resp.body_mut().read_json()?;
        if let Some(e) = search.error {
            // Code 4: "Quota limit exceeded" (50 requests per 5 seconds).
            return Err(if e.code == 4 {
                DeezerError::Quota
            } else {
                DeezerError::Api(e.message)
            });
        }
        Ok(search.data)
    }

    pub fn download(&self, url: &str) -> Result<Vec<u8>, DeezerError> {
        let mut resp = self.agent.get(url).call()?;
        if !resp.status().is_success() {
            return Err(DeezerError::Api(format!("HTTP {}", resp.status())));
        }
        Ok(resp
            .body_mut()
            .with_config()
            .limit(MAX_PICTURE_BYTES)
            .read_to_vec()?)
    }
}

/// How names are compared: lower case, without accents, punctuation or a leading "The",
/// and "&" read as "and" ("Beyoncé" = "Beyonce", "The Beatles" = "Beatles").
pub fn match_key(name: &str) -> String {
    let folded: String = name
        .nfkd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
        .replace('&', " and ");
    let words: Vec<&str> = folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let words = match words.as_slice() {
        ["the", rest @ ..] if !rest.is_empty() => rest,
        all => all,
    };
    words.join(" ")
}

/// Same title, allowing an edition note on either side ("underdogs" = "underdogs (Deluxe)").
pub fn same_title(ours: &str, theirs: &str) -> bool {
    let (a, b) = (match_key(ours), match_key(theirs));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let (short, long) = if a.len() <= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    short == long || (short.len() >= 3 && long.starts_with(&format!("{short} ")))
}

/// The artist of the first hit whose artist is `name` and whose title is `title`.
pub fn verified<'a>(name: &str, title: &str, hits: &'a [Hit]) -> Option<&'a Candidate> {
    let key = match_key(name);
    if key.is_empty() {
        return None;
    }
    hits.iter()
        .find(|h| match_key(&h.artist.name) == key && same_title(title, &h.title))
        .map(|h| &h.artist)
}

/// The largest picture, unless it's Deezer's placeholder for artists without one (an image
/// URL with an empty id: `…/images/artist//1000x1000-….jpg`).
fn picture(c: &Candidate) -> Option<Picture> {
    let url = c.picture_xl.as_deref().or(c.picture_big.as_deref())?;
    (url.starts_with("http") && !url.contains("/artist//")).then(|| Picture {
        artist_id: c.id,
        url: url.to_string(),
    })
}

/// Whether a name stands for someone in particular ("Various Artists" doesn't).
pub fn worth_looking_up(name: &str) -> bool {
    let key = match_key(name);
    !key.is_empty()
        && !matches!(
            key.as_str(),
            "various artists"
                | "various"
                | "va"
                | "unknown"
                | "unknown artist"
                | "artist"
                | "anonymous"
                | "soundtrack"
                | "original soundtrack"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(title: &str, artist_id: u64, artist: &str, xl: &str) -> Hit {
        Hit {
            title: title.into(),
            artist: Candidate {
                id: artist_id,
                name: artist.into(),
                picture_big: None,
                picture_xl: Some(xl.into()),
            },
        }
    }

    #[test]
    fn names_match_ignoring_case_accents_punctuation_and_the() {
        assert_eq!(match_key("Beyoncé"), "beyonce");
        assert_eq!(match_key("The Beatles"), "beatles");
        assert_eq!(match_key("Simon & Garfunkel"), "simon and garfunkel");
        assert_eq!(match_key("AC/DC"), "ac dc");
        assert_eq!(match_key("  Sigur Rós "), "sigur ros");
        assert_eq!(match_key("The The"), "the");
        assert_eq!(match_key("坂本龍一"), "坂本龍一");
    }

    #[test]
    fn titles_match_with_edition_notes_but_not_by_accident() {
        assert!(same_title("underdogs", "Underdogs"));
        assert!(same_title("underdogs", "underdogs (Deluxe Edition)"));
        assert!(same_title(
            "Call Me Nihilist [Remastered]",
            "Call Me Nihilist"
        ));
        assert!(!same_title("underdogs", "underdog"));
        assert!(
            !same_title("Go", "Go Away"),
            "too short to be an edition prefix"
        );
        assert!(!same_title("", "anything"));
    }

    #[test]
    fn an_artist_counts_only_with_one_of_our_titles() {
        // Two artists called Sally; ours made "underdogs".
        let hits = [
            hit("Pop Hits", 1, "Sally", "https://x/famous.jpg"),
            hit("Underdogs", 2, "sally", "https://x/ours.jpg"),
            hit(
                "Underdogs",
                3,
                "Sally Tribute Band",
                "https://x/tribute.jpg",
            ),
        ];
        assert_eq!(verified("Sally", "underdogs", &hits).unwrap().id, 2);
        assert!(verified("Sally", "Call Me Nihilist", &hits).is_none());
        assert!(verified("", "underdogs", &hits).is_none());
    }

    #[test]
    fn deezers_placeholder_is_no_picture() {
        let none = hit(
            "A",
            1,
            "X",
            "https://e-cdns-images.dzcdn.net/images/artist//1000x1000-000000-80-0-0.jpg",
        );
        assert_eq!(picture(&none.artist), None);
        let some = hit(
            "A",
            2,
            "Y",
            "https://e-cdns-images.dzcdn.net/images/artist/f2bc007e9133c946ac3c3907ddc5d2ea/1000x1000-000000-80-0-0.jpg",
        );
        assert_eq!(picture(&some.artist).unwrap().artist_id, 2);
    }

    #[test]
    fn placeholders_for_nobody_in_particular_are_skipped() {
        assert!(!worth_looking_up("Various Artists"));
        assert!(!worth_looking_up("VA"));
        assert!(!worth_looking_up("  "));
        assert!(worth_looking_up("Various Cruelties"));
        assert!(worth_looking_up("System Of A Down"));
    }
}
