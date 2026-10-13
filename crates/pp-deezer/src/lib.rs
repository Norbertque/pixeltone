//! Artist pictures from Deezer's public API (no account or key needed): search an artist by
//! name, keep only an exact match, download its picture. Only the artist's name is sent.

use std::time::Duration;

use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
use ureq::Agent;

pub const API: &str = "https://api.deezer.com";
/// Pictures are about 100–300 KB; anything far bigger isn't one.
const MAX_PICTURE_BYTES: u64 = 8 * 1024 * 1024;

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

/// One artist in Deezer's search results.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Candidate {
    pub id: u64,
    pub name: String,
    pub picture_big: Option<String>,
    pub picture_xl: Option<String>,
    pub nb_fan: u64,
}

#[derive(Deserialize)]
struct Search {
    #[serde(default)]
    data: Vec<Candidate>,
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct ApiError {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    message: String,
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
}

impl Deezer {
    pub fn new(base: &str) -> Self {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .user_agent(concat!(
                "PixelPlayer PC/",
                env!("CARGO_PKG_VERSION"),
                " (https://github.com/Norbertque/pixelplayer-pc)"
            ))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            base: base.trim_end_matches('/').to_string(),
        }
    }

    /// The picture of the artist called `name`; `None` when Deezer doesn't know an artist of
    /// exactly that name, or has no picture of them.
    pub fn find(&self, name: &str) -> Result<Option<Picture>, DeezerError> {
        let mut resp = self
            .agent
            .get(format!("{}/search/artist", self.base))
            .query("q", name)
            .query("limit", "10")
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
        Ok(best_match(name, &search.data).and_then(picture))
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

/// The candidate whose name is the same as `name`; the best known one when several are.
/// Nothing else: a near miss would show the wrong person.
pub fn best_match<'a>(name: &str, candidates: &'a [Candidate]) -> Option<&'a Candidate> {
    let key = match_key(name);
    if key.is_empty() {
        return None;
    }
    candidates
        .iter()
        .rev() // on equal fans, the earlier (more relevant) result wins
        .filter(|c| match_key(&c.name) == key)
        .max_by_key(|c| c.nb_fan)
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

    fn candidate(id: u64, name: &str, fans: u64, xl: &str) -> Candidate {
        Candidate {
            id,
            name: name.into(),
            picture_big: None,
            picture_xl: Some(xl.into()),
            nb_fan: fans,
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
        assert_eq!(match_key("System Of A Down"), match_key("system of a down"));
        assert_eq!(match_key("坂本龍一"), "坂本龍一");
    }

    #[test]
    fn only_an_exact_name_counts_and_fans_break_ties() {
        let found = [
            candidate(1, "Loser64 Tribute", 900, "https://x/a.jpg"),
            candidate(2, "loser64", 50, "https://x/b.jpg"),
            candidate(3, "Loser 64", 70, "https://x/c.jpg"),
            candidate(4, "LOSER64", 10, "https://x/d.jpg"),
        ];
        assert_eq!(best_match("Loser64", &found).unwrap().id, 2);
        assert!(best_match("Loser65", &found).is_none());
        assert!(best_match("", &found).is_none());
        let tie = [
            candidate(7, "Aurora Lane", 5, "https://x/1.jpg"),
            candidate(8, "aurora lane", 5, "https://x/2.jpg"),
        ];
        assert_eq!(best_match("Aurora Lane", &tie).unwrap().id, 7);
    }

    #[test]
    fn deezers_placeholder_is_no_picture() {
        let none = candidate(
            1,
            "X",
            0,
            "https://e-cdns-images.dzcdn.net/images/artist//1000x1000-000000-80-0-0.jpg",
        );
        assert_eq!(picture(&none), None);
        let some = candidate(
            2,
            "Y",
            0,
            "https://e-cdns-images.dzcdn.net/images/artist/f2bc007e9133c946ac3c3907ddc5d2ea/1000x1000-000000-80-0-0.jpg",
        );
        assert_eq!(picture(&some).unwrap().artist_id, 2);
        let big_only = Candidate {
            picture_xl: None,
            picture_big: Some("https://x/500.jpg".into()),
            ..candidate(3, "Z", 0, "")
        };
        assert_eq!(picture(&big_only).unwrap().url, "https://x/500.jpg");
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
