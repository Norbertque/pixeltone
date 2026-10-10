//! The play queue and its repeat rules.

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

#[derive(Clone, Debug, Default)]
pub struct Queue {
    items: Vec<PathBuf>,
    pub repeat: RepeatMode,
}

impl Queue {
    pub fn set(&mut self, items: Vec<PathBuf>) {
        self.items = items;
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, pos: usize) -> Option<&PathBuf> {
        self.items.get(pos)
    }

    /// What plays automatically after `pos` finishes (gapless continuation).
    pub fn after(&self, pos: usize) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        match self.repeat {
            RepeatMode::One => Some(pos.min(self.items.len() - 1)),
            RepeatMode::All => Some((pos + 1) % self.items.len()),
            RepeatMode::Off => (pos + 1 < self.items.len()).then_some(pos + 1),
        }
    }

    /// The user pressed "next": repeat-one doesn't hold them on the same track.
    pub fn next(&self, pos: usize) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        if pos + 1 < self.items.len() {
            Some(pos + 1)
        } else if self.repeat == RepeatMode::All {
            Some(0)
        } else {
            None
        }
    }

    /// The user pressed "previous".
    pub fn previous(&self, pos: usize) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        if pos > 0 {
            Some(pos - 1)
        } else if self.repeat == RepeatMode::All {
            Some(self.items.len() - 1)
        } else {
            Some(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: usize, repeat: RepeatMode) -> Queue {
        Queue {
            items: (0..n).map(|i| PathBuf::from(format!("{i}.flac"))).collect(),
            repeat,
        }
    }

    #[test]
    fn automatic_advance_respects_repeat() {
        assert_eq!(q(3, RepeatMode::Off).after(1), Some(2));
        assert_eq!(q(3, RepeatMode::Off).after(2), None);
        assert_eq!(q(3, RepeatMode::All).after(2), Some(0));
        assert_eq!(q(3, RepeatMode::One).after(1), Some(1));
        assert_eq!(q(0, RepeatMode::All).after(0), None);
    }

    #[test]
    fn manual_skips() {
        assert_eq!(q(3, RepeatMode::One).next(1), Some(2));
        assert_eq!(q(3, RepeatMode::Off).next(2), None);
        assert_eq!(q(3, RepeatMode::All).next(2), Some(0));
        assert_eq!(q(3, RepeatMode::Off).previous(0), Some(0));
        assert_eq!(q(3, RepeatMode::All).previous(0), Some(2));
    }
}
