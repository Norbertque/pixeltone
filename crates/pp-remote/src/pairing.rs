//! Pairing: the PC shows a six-digit code, a phone sends it once and gets a token of its own.
//!
//! A code is single-use, expires after a few minutes and stops working after a few wrong
//! guesses, so it can't be brute-forced. Tokens are random 256-bit values; only their
//! SHA-256 is stored, so the devices file can't be used to impersonate a phone.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// How long a pairing code works.
pub const CODE_VALIDITY: Duration = Duration::from_secs(10 * 60);
/// Wrong guesses before a code stops working.
pub const MAX_FAILURES: u32 = 5;

/// A phone allowed to use the remote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDevice {
    /// Chosen by the phone (stable per installation).
    pub id: String,
    pub name: String,
    /// SHA-256 of the token, hex.
    pub token_hash: String,
    /// Unix seconds.
    pub paired_at: u64,
    pub last_seen: u64,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PairError {
    #[error("Pairing isn't open: show a code on the PC first.")]
    Closed,
    #[error("That code isn't right.")]
    WrongCode,
    #[error("Too many wrong codes: show a new one on the PC.")]
    TooManyTries,
    #[error("{0}")]
    Store(String),
}

struct ActiveCode {
    code: String,
    expires: Instant,
    failures: u32,
}

pub struct Pairing {
    path: PathBuf,
    code: Mutex<Option<ActiveCode>>,
    devices: Mutex<Vec<PairedDevice>>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub(crate) fn random<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("the system's random source");
    bytes
}

pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Comparison that takes as long whether the strings differ early or late.
pub(crate) fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

impl Pairing {
    /// Loads the paired devices from `path` (created when the first phone pairs).
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let devices = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            path,
            code: Mutex::new(None),
            devices: Mutex::new(devices),
        }
    }

    /// A new six-digit code, replacing any earlier one.
    pub fn new_code(&self) -> String {
        // Uniform over 000000–999999: reject the top of the u32 range.
        let code = loop {
            let n = u32::from_le_bytes(random::<4>());
            if n < u32::MAX - u32::MAX % 1_000_000 {
                break format!("{:06}", n % 1_000_000);
            }
        };
        *self.code.lock() = Some(ActiveCode {
            code: code.clone(),
            expires: Instant::now() + CODE_VALIDITY,
            failures: 0,
        });
        code
    }

    /// The code shown now and how long it still works, if pairing is open.
    pub fn current_code(&self) -> Option<(String, Duration)> {
        let code = self.code.lock();
        code.as_ref().and_then(|c| {
            c.expires
                .checked_duration_since(Instant::now())
                .map(|left| (c.code.clone(), left))
        })
    }

    pub fn close(&self) {
        *self.code.lock() = None;
    }

    /// Pairs a phone with the code it was shown; returns its new token.
    pub fn pair(
        &self,
        code: &str,
        device_id: &str,
        device_name: &str,
    ) -> Result<String, PairError> {
        {
            let mut active = self.code.lock();
            let Some(current) = active.as_mut() else {
                return Err(PairError::Closed);
            };
            if Instant::now() > current.expires {
                *active = None;
                return Err(PairError::Closed);
            }
            if !same(code.trim(), &current.code) {
                current.failures += 1;
                if current.failures >= MAX_FAILURES {
                    *active = None;
                    return Err(PairError::TooManyTries);
                }
                return Err(PairError::WrongCode);
            }
            // Single use.
            *active = None;
        }
        let token = URL_SAFE_NO_PAD.encode(random::<32>());
        let device = PairedDevice {
            id: clip(device_id, 64),
            name: clip(device_name, 64),
            token_hash: hash_token(&token),
            paired_at: now(),
            last_seen: now(),
        };
        let mut devices = self.devices.lock();
        devices.retain(|d| d.id != device.id);
        devices.push(device);
        save(&self.path, &devices).map_err(PairError::Store)?;
        Ok(token)
    }

    /// The device a token belongs to; notes when it was last seen.
    pub fn authorize(&self, token: &str) -> Option<PairedDevice> {
        let hash = hash_token(token);
        let mut devices = self.devices.lock();
        let device = devices.iter_mut().find(|d| same(&d.token_hash, &hash))?;
        // Saved at most hourly: this runs on every request.
        let seen = now();
        let stale = seen.saturating_sub(device.last_seen) > 3600;
        device.last_seen = seen;
        let found = device.clone();
        if stale {
            let _ = save(&self.path, &devices);
        }
        Some(found)
    }

    pub fn devices(&self) -> Vec<PairedDevice> {
        self.devices.lock().clone()
    }

    pub fn forget(&self, device_id: &str) -> Result<(), String> {
        let mut devices = self.devices.lock();
        devices.retain(|d| d.id != device_id);
        save(&self.path, &devices)
    }
}

fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    let s = if s.is_empty() { "Phone" } else { s };
    s.chars().take(max).collect()
}

fn save(path: &Path, devices: &[PairedDevice]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(devices).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_pairs_once_and_the_token_authorizes() {
        let dir = tempfile::tempdir().unwrap();
        let p = Pairing::open(dir.path().join("devices.json"));
        assert_eq!(p.pair("123456", "a", "Phone"), Err(PairError::Closed));

        let code = p.new_code();
        assert_eq!(code.len(), 6);
        assert!(code.bytes().all(|b| b.is_ascii_digit()));
        let token = p.pair(&format!(" {code} "), "pixel-8", "Pixel 8").unwrap();
        assert!(token.len() >= 43, "256 bits");
        assert_eq!(
            p.pair(&code, "other", "Other"),
            Err(PairError::Closed),
            "single use"
        );

        let device = p.authorize(&token).unwrap();
        assert_eq!(device.name, "Pixel 8");
        assert!(p.authorize("nope").is_none());

        // The token itself is never stored.
        let stored = std::fs::read_to_string(dir.path().join("devices.json")).unwrap();
        assert!(!stored.contains(&token));
        assert!(
            Pairing::open(dir.path().join("devices.json"))
                .authorize(&token)
                .is_some()
        );

        p.forget("pixel-8").unwrap();
        assert!(p.authorize(&token).is_none());
    }

    #[test]
    fn wrong_guesses_close_the_code() {
        let dir = tempfile::tempdir().unwrap();
        let p = Pairing::open(dir.path().join("devices.json"));
        let code = p.new_code();
        let wrong = if code == "000000" { "000001" } else { "000000" };
        for _ in 1..MAX_FAILURES {
            assert_eq!(p.pair(wrong, "x", "X"), Err(PairError::WrongCode));
        }
        assert_eq!(p.pair(wrong, "x", "X"), Err(PairError::TooManyTries));
        assert_eq!(
            p.pair(&code, "x", "X"),
            Err(PairError::Closed),
            "even the right code now"
        );
    }

    #[test]
    fn pairing_again_replaces_the_old_token() {
        let dir = tempfile::tempdir().unwrap();
        let p = Pairing::open(dir.path().join("devices.json"));
        let first = p.pair(&p.new_code(), "phone", "Phone").unwrap();
        let second = p.pair(&p.new_code(), "phone", "Phone").unwrap();
        assert!(p.authorize(&first).is_none());
        assert!(p.authorize(&second).is_some());
        assert_eq!(p.devices().len(), 1);
    }
}
