//! A jam: a temporary shared queue on this PC that guests join with its QR code or link
//! (`GLOSSARY.md`, ADR 0002). Guests are never paired: their tokens live here, in memory, and
//! die with the jam. The owner (the person at the PC and every paired phone) starts and ends
//! it, sets the guest permissions, removes guests and answers their rejoin requests.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

use crate::pairing::{hash_token, random, same};

/// Most guests in one jam.
pub const MAX_GUESTS: usize = 32;
/// How long a rejoin request waits for the owner's answer.
pub const REQUEST_VALIDITY: Duration = Duration::from_secs(2 * 60);
/// A guest's name is cut to this many characters.
const MAX_NAME: usize = 40;

/// What guests may do in a jam; the owner changes it while the jam runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestPermissions {
    pub add_tracks: bool,
    /// Remove and move the tracks they added.
    pub change_own_tracks: bool,
    /// Play, pause, skip and seek.
    pub control_playback: bool,
}

impl Default for GuestPermissions {
    fn default() -> Self {
        Self {
            add_tracks: true,
            change_own_tracks: true,
            control_playback: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Guest {
    pub id: String,
    pub name: String,
    /// The phone's own id: a removed guest is known by it when they come back.
    pub device_id: String,
    token_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestInfo {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejoinRequestInfo {
    pub id: String,
    pub name: String,
}

/// The jam as the owner sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JamInfo {
    pub permissions: GuestPermissions,
    pub guests: Vec<GuestInfo>,
    /// Removed guests asking to come back, waiting for an answer.
    pub requests: Vec<RejoinRequestInfo>,
}

/// How joining (or asking to come back) went.
#[derive(Debug, PartialEq, Eq)]
pub enum Join {
    /// In: `token` is the guest's key to this jam.
    Joined {
        guest_id: String,
        token: String,
    },
    /// A removed guest asking to come back: the owner decides. Ask again with `request_id`.
    Waiting {
        request_id: String,
    },
    /// The owner said no; that holds for the rest of this jam.
    Declined,
    Full,
    /// Not this jam's QR code (an old one, from an ended jam).
    WrongSecret,
}

struct Request {
    id: String,
    name: String,
    device_id: String,
    asked: Instant,
    answer: Option<Answer>,
}

enum Answer {
    Allowed { guest_id: String, token: String },
    Declined,
}

pub struct Jam {
    /// Names the jam where that's no secret (the folder of its phone tracks).
    id: String,
    secret: String,
    pub permissions: GuestPermissions,
    guests: Vec<Guest>,
    /// Phones removed from this jam: they get back in only through a rejoin request.
    removed: HashSet<String>,
    /// Phones whose rejoin request was declined: final for this jam.
    declined: HashSet<String>,
    requests: Vec<Request>,
}

fn new_token() -> String {
    URL_SAFE_NO_PAD.encode(random::<32>())
}

fn new_id() -> String {
    URL_SAFE_NO_PAD.encode(random::<9>())
}

/// Trimmed, single-spaced, at most [`MAX_NAME`] characters; "Guest" when empty.
fn clean_name(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let joined: String = words.join(" ").chars().take(MAX_NAME).collect();
    if joined.is_empty() {
        "Guest".into()
    } else {
        joined
    }
}

impl Default for Jam {
    fn default() -> Self {
        Self::new()
    }
}

impl Jam {
    pub fn new() -> Self {
        Self {
            id: new_id(),
            secret: new_token(),
            permissions: GuestPermissions::default(),
            guests: Vec::new(),
            removed: HashSet::new(),
            declined: HashSet::new(),
            requests: Vec::new(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// What the QR code carries, besides the PC's address and certificate.
    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn join(&mut self, secret: &str, device_id: &str, name: &str, now: Instant) -> Join {
        if !same(secret, &self.secret) {
            return Join::WrongSecret;
        }
        self.expire(now);
        if self.declined.contains(device_id) {
            return Join::Declined;
        }
        if self.removed.contains(device_id) {
            // A request already made counts, answered or not.
            if let Some(id) = self
                .requests
                .iter()
                .find(|r| r.device_id == device_id)
                .map(|r| r.id.clone())
            {
                return self
                    .poll(&id, now)
                    .unwrap_or(Join::Waiting { request_id: id });
            }
            let id = new_id();
            self.requests.push(Request {
                id: id.clone(),
                name: clean_name(name),
                device_id: device_id.into(),
                asked: now,
                answer: None,
            });
            return Join::Waiting { request_id: id };
        }
        // The same phone again (the app lost its token, say): a new token, the same guest.
        if let Some(guest) = self.guests.iter_mut().find(|g| g.device_id == device_id) {
            let token = new_token();
            guest.token_hash = hash_token(&token);
            guest.name = clean_name(name);
            return Join::Joined {
                guest_id: guest.id.clone(),
                token,
            };
        }
        if self.guests.len() >= MAX_GUESTS {
            return Join::Full;
        }
        let (guest_id, token) = self.add_guest(device_id, name);
        Join::Joined { guest_id, token }
    }

    /// Where a rejoin request stands; `None` once it expired unanswered (ask again).
    pub fn poll(&mut self, request_id: &str, now: Instant) -> Option<Join> {
        self.expire(now);
        let index = self.requests.iter().position(|r| r.id == request_id)?;
        let outcome = match &self.requests[index].answer {
            None => {
                return Some(Join::Waiting {
                    request_id: request_id.into(),
                });
            }
            Some(Answer::Allowed { guest_id, token }) => Join::Joined {
                guest_id: guest_id.clone(),
                token: token.clone(),
            },
            Some(Answer::Declined) => Join::Declined,
        };
        // Handed over once: the token isn't kept around in the request.
        self.requests.remove(index);
        Some(outcome)
    }

    /// The owner's answer to a rejoin request. False when there's no such request waiting
    /// (expired, answered already) or when allowing it would go over [`MAX_GUESTS`].
    pub fn answer(&mut self, request_id: &str, allow: bool, now: Instant) -> bool {
        self.expire(now);
        let Some(index) = self
            .requests
            .iter()
            .position(|r| r.id == request_id && r.answer.is_none())
        else {
            return false;
        };
        let device_id = self.requests[index].device_id.clone();
        if allow {
            if self.guests.len() >= MAX_GUESTS {
                return false;
            }
            self.removed.remove(&device_id);
            let name = self.requests[index].name.clone();
            let (guest_id, token) = self.add_guest(&device_id, &name);
            self.requests[index].answer = Some(Answer::Allowed { guest_id, token });
        } else {
            self.declined.insert(device_id);
            self.requests[index].answer = Some(Answer::Declined);
        }
        true
    }

    /// Takes a guest out of the jam: their token stops working, and coming back takes a rejoin
    /// request.
    pub fn remove(&mut self, guest_id: &str) -> Option<Guest> {
        let index = self.guests.iter().position(|g| g.id == guest_id)?;
        let guest = self.guests.remove(index);
        self.removed.insert(guest.device_id.clone());
        Some(guest)
    }

    /// A guest leaving on their own; they may join again like anyone.
    pub fn leave(&mut self, guest_id: &str) -> Option<Guest> {
        let index = self.guests.iter().position(|g| g.id == guest_id)?;
        Some(self.guests.remove(index))
    }

    /// The guest a token belongs to.
    pub fn guest(&self, token: &str) -> Option<&Guest> {
        let hash = hash_token(token);
        self.guests.iter().find(|g| same(&g.token_hash, &hash))
    }

    pub fn guest_by_id(&self, guest_id: &str) -> Option<&Guest> {
        self.guests.iter().find(|g| g.id == guest_id)
    }

    pub fn info(&mut self, now: Instant) -> JamInfo {
        self.expire(now);
        JamInfo {
            permissions: self.permissions,
            guests: self
                .guests
                .iter()
                .map(|g| GuestInfo {
                    id: g.id.clone(),
                    name: g.name.clone(),
                })
                .collect(),
            requests: self
                .requests
                .iter()
                .filter(|r| r.answer.is_none())
                .map(|r| RejoinRequestInfo {
                    id: r.id.clone(),
                    name: r.name.clone(),
                })
                .collect(),
        }
    }

    fn add_guest(&mut self, device_id: &str, name: &str) -> (String, String) {
        let token = new_token();
        let id = new_id();
        self.guests.push(Guest {
            id: id.clone(),
            name: clean_name(name),
            device_id: device_id.into(),
            token_hash: hash_token(&token),
        });
        (id, token)
    }

    /// Drops unanswered requests older than [`REQUEST_VALIDITY`] (answered ones wait to be
    /// picked up).
    fn expire(&mut self, now: Instant) {
        self.requests.retain(|r| {
            r.answer.is_some() || now.saturating_duration_since(r.asked) < REQUEST_VALIDITY
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(outcome: Join) -> (String, String) {
        match outcome {
            Join::Joined { guest_id, token } => (guest_id, token),
            other => panic!("expected Joined, got {other:?}"),
        }
    }

    fn waiting(outcome: Join) -> String {
        match outcome {
            Join::Waiting { request_id } => request_id,
            other => panic!("expected Waiting, got {other:?}"),
        }
    }

    #[test]
    fn guests_join_with_the_secret_and_get_a_token() {
        let mut jam = Jam::new();
        let now = Instant::now();
        assert_eq!(
            jam.join("not it", "phone-a", "Petr", now),
            Join::WrongSecret
        );

        let secret = jam.secret().to_owned();
        let (id, token) = joined(jam.join(&secret, "phone-a", "  Petr   Novák ", now));
        assert_eq!(
            jam.guest(&token).map(|g| g.name.as_str()),
            Some("Petr Novák")
        );
        assert!(jam.guest("someone else's").is_none());

        // The same phone again: same guest, new token, the old one gone.
        let (again, new_token) = joined(jam.join(&secret, "phone-a", "Petr", now));
        assert_eq!(again, id);
        assert!(jam.guest(&token).is_none());
        assert!(jam.guest(&new_token).is_some());
        assert_eq!(jam.info(now).guests.len(), 1);
    }

    #[test]
    fn a_jam_holds_at_most_32_guests() {
        let mut jam = Jam::new();
        let now = Instant::now();
        let secret = jam.secret().to_owned();
        for i in 0..MAX_GUESTS {
            joined(jam.join(&secret, &format!("phone-{i}"), "", now));
        }
        assert_eq!(jam.join(&secret, "one-more", "Late", now), Join::Full);
        assert_eq!(jam.info(now).guests[0].name, "Guest");
    }

    #[test]
    fn a_removed_guest_comes_back_only_if_the_owner_allows() {
        let mut jam = Jam::new();
        let now = Instant::now();
        let secret = jam.secret().to_owned();
        let (id, token) = joined(jam.join(&secret, "phone-a", "Petr", now));
        assert!(jam.remove(&id).is_some());
        assert!(jam.guest(&token).is_none());

        let request = waiting(jam.join(&secret, "phone-a", "Petr", now));
        assert_eq!(
            jam.info(now).requests,
            vec![RejoinRequestInfo {
                id: request.clone(),
                name: "Petr".into()
            }]
        );
        // Asking again while it waits is the same request.
        assert_eq!(waiting(jam.join(&secret, "phone-a", "Petr", now)), request);
        assert_eq!(
            jam.poll(&request, now),
            Some(Join::Waiting {
                request_id: request.clone()
            })
        );

        assert!(jam.answer(&request, true, now));
        assert!(!jam.answer(&request, true, now), "answered already");
        let (_, token) = joined(jam.poll(&request, now).unwrap());
        assert!(jam.guest(&token).is_some());
        assert!(jam.info(now).requests.is_empty());
        // Back as an ordinary guest: joining again needs no request.
        joined(jam.join(&secret, "phone-a", "Petr", now));
    }

    #[test]
    fn a_declined_guest_stays_out_of_this_jam() {
        let mut jam = Jam::new();
        let now = Instant::now();
        let secret = jam.secret().to_owned();
        let (id, _) = joined(jam.join(&secret, "phone-a", "Petr", now));
        jam.remove(&id);
        let request = waiting(jam.join(&secret, "phone-a", "Petr", now));
        assert!(jam.answer(&request, false, now));
        assert_eq!(jam.poll(&request, now), Some(Join::Declined));
        assert_eq!(jam.join(&secret, "phone-a", "Petr", now), Join::Declined);
        // A new jam starts afresh.
        let mut next = Jam::new();
        let secret = next.secret().to_owned();
        joined(next.join(&secret, "phone-a", "Petr", now));
    }

    #[test]
    fn unanswered_requests_expire_after_two_minutes() {
        let mut jam = Jam::new();
        let now = Instant::now();
        let secret = jam.secret().to_owned();
        let (id, _) = joined(jam.join(&secret, "phone-a", "Petr", now));
        jam.remove(&id);
        let request = waiting(jam.join(&secret, "phone-a", "Petr", now));
        let later = now + REQUEST_VALIDITY;
        assert_eq!(jam.poll(&request, later), None);
        assert!(!jam.answer(&request, true, later));
        assert!(jam.info(later).requests.is_empty());
        // They may ask again.
        let again = waiting(jam.join(&secret, "phone-a", "Petr", later));
        assert_ne!(again, request);
    }

    #[test]
    fn a_guest_who_left_may_join_again() {
        let mut jam = Jam::new();
        let now = Instant::now();
        let secret = jam.secret().to_owned();
        let (id, token) = joined(jam.join(&secret, "phone-a", "Petr", now));
        assert!(jam.leave(&id).is_some());
        assert!(jam.guest(&token).is_none());
        joined(jam.join(&secret, "phone-a", "Petr", now));
    }
}
