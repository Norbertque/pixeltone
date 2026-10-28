//! The play queue: play order, the original order while shuffled, and the repeat rules.
//!
//! Items carry ids that stay stable while they are queued, so the decode worker, the engine
//! and the UI keep pointing at the right track while the queue is edited underneath them.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

/// A track to queue, with what the queue list shows before the file is opened.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QueueItem {
    pub path: PathBuf,
    /// Cover to show instead of the file's embedded picture (a library thumbnail key).
    pub cover_key: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<f64>,
    /// Library track id, when the item came from the library.
    pub track_id: Option<u32>,
    /// `path` is an `http://` address a paired phone serves (remote control), not a file.
    pub remote: bool,
    /// The guest who added it, in a jam.
    pub added_by: Option<String>,
    /// That guest's id in the jam: only they may change it.
    pub guest_id: Option<String>,
}

impl From<PathBuf> for QueueItem {
    fn from(path: PathBuf) -> Self {
        Self {
            path,
            ..Self::default()
        }
    }
}

/// One queue row as the UI lists it.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct QueueEntry {
    pub id: u32,
    pub path: String,
    /// The tag title, or the file name.
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<f64>,
    pub cover_key: Option<String>,
    pub track_id: Option<u32>,
    /// The guest who added it, in a jam.
    pub added_by: Option<String>,
    /// That guest's id in the jam.
    pub guest_id: Option<String>,
}

/// Everything needed to rebuild the queue after a restart.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QueueSnapshot {
    /// In play order.
    pub items: Vec<QueueItem>,
    /// While shuffled: indices into `items` in the original (unshuffled) order.
    pub original: Option<Vec<u32>>,
    /// Index of the current track in `items`.
    pub current: Option<u32>,
    pub position_ms: f64,
    pub repeat: RepeatMode,
}

#[derive(Clone, Debug)]
struct Entry {
    id: u32,
    item: QueueItem,
}

#[derive(Clone, Debug, Default)]
pub struct Queue {
    /// Play order.
    entries: Vec<Entry>,
    /// Ids in the original order while shuffled.
    original: Option<Vec<u32>>,
    pub repeat: RepeatMode,
    last_id: u32,
    /// Bumped whenever the entries or their order change.
    version: u32,
}

impl Queue {
    fn assign(&mut self, items: Vec<QueueItem>) -> Vec<Entry> {
        items
            .into_iter()
            .map(|item| {
                self.last_id = self.last_id.wrapping_add(1);
                Entry {
                    id: self.last_id,
                    item,
                }
            })
            .collect()
    }

    fn changed(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    /// Replaces the queue (unshuffled); returns the new ids in order.
    pub fn set(&mut self, items: Vec<QueueItem>) -> Vec<u32> {
        self.entries = self.assign(items);
        self.original = None;
        self.changed();
        self.entries.iter().map(|e| e.id).collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn is_shuffled(&self) -> bool {
        self.original.is_some()
    }

    pub fn index_of(&self, id: u32) -> Option<usize> {
        self.entries.iter().position(|e| e.id == id)
    }

    pub fn get(&self, id: u32) -> Option<&QueueItem> {
        self.entries.iter().find(|e| e.id == id).map(|e| &e.item)
    }

    pub fn id_at(&self, index: usize) -> Option<u32> {
        self.entries.get(index).map(|e| e.id)
    }

    /// What plays automatically after `id` finishes (gapless continuation).
    pub fn after(&self, id: u32) -> Option<u32> {
        let i = self.index_of(id)?;
        let n = self.entries.len();
        let next = match self.repeat {
            RepeatMode::One => i,
            RepeatMode::All => (i + 1) % n,
            RepeatMode::Off if i + 1 < n => i + 1,
            RepeatMode::Off => return None,
        };
        self.id_at(next)
    }

    /// The user pressed "next": repeat-one doesn't hold them on the same track.
    pub fn next(&self, id: u32) -> Option<u32> {
        let i = self.index_of(id)?;
        if i + 1 < self.entries.len() {
            self.id_at(i + 1)
        } else if self.repeat == RepeatMode::All {
            self.id_at(0)
        } else {
            None
        }
    }

    /// The user pressed "previous".
    pub fn previous(&self, id: u32) -> Option<u32> {
        let i = self.index_of(id)?;
        if i > 0 {
            self.id_at(i - 1)
        } else if self.repeat == RepeatMode::All {
            self.id_at(self.entries.len() - 1)
        } else {
            Some(id)
        }
    }

    /// "Play next": inserts right after `anchor` (at the front without one).
    pub fn insert_after(&mut self, anchor: Option<u32>, items: Vec<QueueItem>) -> Vec<u32> {
        let new = self.assign(items);
        let ids: Vec<u32> = new.iter().map(|e| e.id).collect();
        let at = anchor.and_then(|a| self.index_of(a)).map_or(0, |i| i + 1);
        self.entries.splice(at..at, new);
        if let Some(order) = &mut self.original {
            let at = anchor
                .and_then(|a| order.iter().position(|&x| x == a))
                .map_or(0, |i| i + 1);
            order.splice(at..at, ids.iter().copied());
        }
        self.changed();
        ids
    }

    /// "Add to queue": appends at the end.
    pub fn append(&mut self, items: Vec<QueueItem>) -> Vec<u32> {
        let new = self.assign(items);
        let ids: Vec<u32> = new.iter().map(|e| e.id).collect();
        self.entries.extend(new);
        if let Some(order) = &mut self.original {
            order.extend(&ids);
        }
        self.changed();
        ids
    }

    /// Removes items, except `keep` (the track that is playing). Returns how many went.
    pub fn remove(&mut self, ids: &[u32], keep: Option<u32>) -> usize {
        let gone: HashSet<u32> = ids.iter().copied().filter(|&id| Some(id) != keep).collect();
        let before = self.entries.len();
        self.entries.retain(|e| !gone.contains(&e.id));
        if let Some(order) = &mut self.original {
            order.retain(|id| !gone.contains(id));
        }
        let removed = before - self.entries.len();
        if removed > 0 {
            self.changed();
        }
        removed
    }

    /// Moves an item to `index` of the play order.
    pub fn move_to(&mut self, id: u32, index: usize) -> bool {
        let Some(from) = self.index_of(id) else {
            return false;
        };
        let entry = self.entries.remove(from);
        let to = index.min(self.entries.len());
        self.entries.insert(to, entry);
        if from != to {
            self.changed();
        }
        true
    }

    /// Removes everything except `keep`.
    pub fn clear_except(&mut self, keep: Option<u32>) {
        self.entries.retain(|e| Some(e.id) == keep);
        if let Some(order) = &mut self.original {
            order.retain(|&id| Some(id) == keep);
        }
        self.changed();
    }

    /// Shuffles the play order; `first` (the track playing) leads and everything else
    /// follows in random order. The original order is kept for [`Queue::unshuffle`].
    pub fn shuffle(&mut self, first: Option<u32>, seed: u64) {
        if self.original.is_none() {
            self.original = Some(self.entries.iter().map(|e| e.id).collect());
        }
        let mut rng = XorShift::new(seed);
        for i in (1..self.entries.len()).rev() {
            self.entries.swap(i, rng.below(i + 1));
        }
        if let Some(i) = first.and_then(|f| self.index_of(f)) {
            let entry = self.entries.remove(i);
            self.entries.insert(0, entry);
        }
        self.changed();
    }

    /// Restores the original order.
    pub fn unshuffle(&mut self) {
        let Some(order) = self.original.take() else {
            return;
        };
        let mut by_id: HashMap<u32, Entry> = self.entries.drain(..).map(|e| (e.id, e)).collect();
        self.entries = order.iter().filter_map(|id| by_id.remove(id)).collect();
        let mut rest: Vec<Entry> = by_id.into_values().collect();
        rest.sort_by_key(|e| e.id);
        self.entries.extend(rest);
        self.changed();
    }

    /// Rows `offset..offset + limit` of the play order.
    pub fn entries(&self, offset: usize, limit: usize) -> Vec<QueueEntry> {
        self.entries
            .iter()
            .skip(offset)
            .take(limit)
            .map(|e| QueueEntry {
                id: e.id,
                path: e.item.path.display().to_string(),
                title: e.item.title.clone().unwrap_or_else(|| {
                    e.item
                        .path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default()
                }),
                artist: e.item.artist.clone(),
                album: e.item.album.clone(),
                duration_ms: e.item.duration_ms,
                cover_key: e.item.cover_key.clone(),
                track_id: e.item.track_id,
                added_by: e.item.added_by.clone(),
                guest_id: e.item.guest_id.clone(),
            })
            .collect()
    }

    pub fn snapshot(&self, current: Option<u32>, position_ms: f64) -> QueueSnapshot {
        let index: HashMap<u32, u32> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id, i as u32))
            .collect();
        QueueSnapshot {
            items: self.entries.iter().map(|e| e.item.clone()).collect(),
            original: self.original.as_ref().map(|order| {
                order
                    .iter()
                    .filter_map(|id| index.get(id).copied())
                    .collect()
            }),
            current: current.and_then(|id| index.get(&id).copied()),
            position_ms,
            repeat: self.repeat,
        }
    }

    /// Rebuilds the queue from a snapshot; returns the id of its current track.
    pub fn restore(&mut self, snapshot: QueueSnapshot) -> Option<u32> {
        let ids = self.set(snapshot.items);
        self.repeat = snapshot.repeat;
        if let Some(order) = snapshot.original {
            let mut seen = HashSet::new();
            let valid = order.len() == ids.len()
                && order
                    .iter()
                    .all(|&i| (i as usize) < ids.len() && seen.insert(i));
            if valid {
                self.original = Some(order.iter().map(|&i| ids[i as usize]).collect());
            }
        }
        snapshot.current.and_then(|i| ids.get(i as usize).copied())
    }
}

/// Small PRNG for shuffling (xorshift64).
struct XorShift(u64);

impl XorShift {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn below(&mut self, n: usize) -> usize {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x % n as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(names: &[&str]) -> Vec<QueueItem> {
        names
            .iter()
            .map(|n| QueueItem::from(PathBuf::from(format!("{n}.flac"))))
            .collect()
    }

    fn q(n: usize, repeat: RepeatMode) -> (Queue, Vec<u32>) {
        let mut q = Queue::default();
        let names: Vec<String> = (0..n).map(|i| i.to_string()).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let ids = q.set(items(&refs));
        q.repeat = repeat;
        (q, ids)
    }

    fn titles(q: &Queue) -> Vec<String> {
        q.entries(0, usize::MAX)
            .into_iter()
            .map(|e| e.title)
            .collect()
    }

    #[test]
    fn automatic_advance_respects_repeat() {
        let (off, ids) = q(3, RepeatMode::Off);
        assert_eq!(off.after(ids[1]), Some(ids[2]));
        assert_eq!(off.after(ids[2]), None);
        let (all, ids) = q(3, RepeatMode::All);
        assert_eq!(all.after(ids[2]), Some(ids[0]));
        let (one, ids) = q(3, RepeatMode::One);
        assert_eq!(one.after(ids[1]), Some(ids[1]));
        assert_eq!(off.after(999), None);
    }

    #[test]
    fn manual_skips() {
        let (one, ids) = q(3, RepeatMode::One);
        assert_eq!(one.next(ids[1]), Some(ids[2]));
        let (off, ids) = q(3, RepeatMode::Off);
        assert_eq!(off.next(ids[2]), None);
        assert_eq!(off.previous(ids[0]), Some(ids[0]));
        let (all, ids) = q(3, RepeatMode::All);
        assert_eq!(all.next(ids[2]), Some(ids[0]));
        assert_eq!(all.previous(ids[0]), Some(ids[2]));
    }

    #[test]
    fn play_next_and_add_to_queue() {
        let (mut q, ids) = q(3, RepeatMode::Off);
        let v = q.version();
        q.insert_after(Some(ids[0]), items(&["a", "b"]));
        q.append(items(&["z"]));
        assert_eq!(titles(&q), ["0", "a", "b", "1", "2", "z"]);
        assert_ne!(q.version(), v);
        // Ids survive the edits.
        assert_eq!(
            q.after(ids[0]).and_then(|id| q.get(id)).unwrap().path,
            PathBuf::from("a.flac")
        );
        q.insert_after(None, items(&["first"]));
        assert_eq!(titles(&q)[0], "first");
    }

    #[test]
    fn remove_never_takes_the_playing_track() {
        let (mut q, ids) = q(4, RepeatMode::Off);
        assert_eq!(q.remove(&[ids[1], ids[2], 999], Some(ids[1])), 1);
        assert_eq!(titles(&q), ["0", "1", "3"]);
    }

    #[test]
    fn move_and_clear() {
        let (mut q, ids) = q(4, RepeatMode::Off);
        assert!(q.move_to(ids[3], 0));
        assert_eq!(titles(&q), ["3", "0", "1", "2"]);
        assert!(q.move_to(ids[3], 99));
        assert_eq!(titles(&q), ["0", "1", "2", "3"]);
        assert!(!q.move_to(999, 0));
        q.clear_except(Some(ids[2]));
        assert_eq!(titles(&q), ["2"]);
    }

    #[test]
    fn shuffle_keeps_the_current_track_first_and_unshuffle_restores_order() {
        let (mut q, ids) = q(50, RepeatMode::Off);
        q.shuffle(Some(ids[20]), 42);
        assert!(q.is_shuffled());
        let shuffled = titles(&q);
        assert_eq!(shuffled[0], "20");
        assert_ne!(shuffled, (0..50).map(|i| i.to_string()).collect::<Vec<_>>());
        let mut sorted = shuffled.clone();
        sorted.sort_by_key(|t| t.parse::<u32>().unwrap());
        assert_eq!(sorted, (0..50).map(|i| i.to_string()).collect::<Vec<_>>());

        // Edits while shuffled land sensibly in the original order too.
        q.insert_after(Some(ids[20]), items(&["next"]));
        q.append(items(&["end"]));
        q.remove(&[ids[5]], Some(ids[20]));
        q.unshuffle();
        let restored = titles(&q);
        assert!(!q.is_shuffled());
        // "5" is gone, so "19" moved up to index 18.
        assert_eq!(&restored[18..21], ["19", "20", "next"]);
        assert_eq!(restored.last().unwrap(), "end");
        assert!(!restored.contains(&"5".to_string()));
        assert_eq!(restored.len(), 51);
    }

    #[test]
    fn snapshot_round_trip_keeps_shuffle_and_current() {
        let (mut q, ids) = q(10, RepeatMode::All);
        q.shuffle(Some(ids[4]), 7);
        let snap = q.snapshot(Some(ids[4]), 1234.0);
        assert_eq!(snap.current, Some(0));

        let mut restored = Queue::default();
        let current = restored.restore(snap.clone());
        assert_eq!(titles(&restored), titles(&q));
        assert_eq!(
            current.and_then(|id| restored.get(id)).unwrap().path,
            PathBuf::from("4.flac")
        );
        assert_eq!(restored.repeat, RepeatMode::All);
        restored.unshuffle();
        assert_eq!(
            titles(&restored),
            (0..10).map(|i| i.to_string()).collect::<Vec<_>>()
        );

        // A damaged original order is ignored rather than trusted.
        let mut bad = snap;
        bad.original = Some(vec![0, 0, 1]);
        let mut q2 = Queue::default();
        q2.restore(bad);
        assert!(!q2.is_shuffled());
    }
}
