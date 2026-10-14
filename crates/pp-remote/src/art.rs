//! Cover pictures of a phone's tracks: fetched from the phone (the URLs it gave with the
//! tracks) into the engine's cover store just before they're needed, so Now Playing, the
//! mini player and the Windows media flyout show them like any other cover.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use parking_lot::Mutex;
use pp_decode::{CoverArt, image_mime};
use pp_engine::CoverStore;
use ureq::Agent;

/// Cover keys of phone tracks start with this.
pub const PREFIX: &str = "remote/";
const MAX_BYTES: u64 = 10 * 1024 * 1024;

pub struct Art {
    agent: Agent,
    urls: Mutex<HashMap<String, String>>,
    /// Keys being fetched right now (by another thread).
    fetching: Mutex<HashSet<String>>,
}

impl Art {
    pub fn new() -> Self {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(4)))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            urls: Mutex::new(HashMap::new()),
            fetching: Mutex::new(HashSet::new()),
        }
    }

    /// A key for a picture at `url` (the same picture gets the same key).
    pub fn key_for(&self, url: &str) -> String {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        // The query (tokens, cache busters) changes per session; the path names the song.
        for b in url.split('?').next().unwrap_or(url).bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        let key = format!("{PREFIX}{h:016x}");
        self.urls.lock().insert(key.clone(), url.to_string());
        key
    }

    /// Fetches the picture behind `key` into `covers` unless it's there. Blocks: call it
    /// off async code. A failed fetch is tried again the next time the picture is needed.
    pub fn ensure(&self, key: &str, covers: &CoverStore) {
        if covers.get(key).is_some() {
            return;
        }
        let Some(url) = self.urls.lock().get(key).cloned() else {
            return;
        };
        if !self.fetching.lock().insert(key.to_string()) {
            return;
        }
        if let Some(art) = self.fetch(&url) {
            covers.insert(key.to_string(), art);
        }
        self.fetching.lock().remove(key);
    }

    fn fetch(&self, url: &str) -> Option<CoverArt> {
        let mut resp = self.agent.get(url).call().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let data = resp
            .body_mut()
            .with_config()
            .limit(MAX_BYTES)
            .read_to_vec()
            .ok()?;
        let mime = image_mime(&data)?;
        Some(CoverArt { data, mime })
    }
}
