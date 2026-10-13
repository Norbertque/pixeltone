//! The tag editor: reads the fields people edit and writes changes back into the files,
//! through `lofty` (FLAC, MP3, M4A/ALAC, Ogg, Opus, WavPack, APE, AIFF, WAV, …). Only the
//! file's main tag is rewritten; covers and every other field in it are kept.

use std::path::Path;

use lofty::config::WriteOptions;
use lofty::file::TaggedFile;
use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The editable fields of a file. As a change, `None` leaves a field as it is and an empty
/// string removes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct TagFields {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<String>,
    pub track_no: Option<String>,
    pub disc_no: Option<String>,
}

fn open(path: &Path) -> Result<TaggedFile, String> {
    lofty::read_from_path(path).map_err(|e| {
        if e.is_unknown_format() {
            "tags of this kind of file can't be edited".to_string()
        } else {
            e.to_string()
        }
    })
}

fn text(tag: &Tag, keys: &[ItemKey]) -> String {
    keys.iter()
        .map(|k| tag.get_strings(*k).collect::<Vec<_>>().join("; "))
        .find(|s| !s.trim().is_empty())
        .unwrap_or_default()
}

/// The file's current values (empty strings for missing fields).
pub fn read_fields(path: &Path) -> Result<TagFields, String> {
    let file = open(path)?;
    let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) else {
        return Ok(TagFields {
            title: Some(String::new()),
            artist: Some(String::new()),
            album: Some(String::new()),
            album_artist: Some(String::new()),
            genre: Some(String::new()),
            year: Some(String::new()),
            track_no: Some(String::new()),
            disc_no: Some(String::new()),
        });
    };
    let number = |n: Option<u32>| {
        n.filter(|&n| n > 0)
            .map(|n| n.to_string())
            .unwrap_or_default()
    };
    Ok(TagFields {
        title: Some(text(tag, &[ItemKey::TrackTitle])),
        artist: Some(text(tag, &[ItemKey::TrackArtists, ItemKey::TrackArtist])),
        album: Some(text(tag, &[ItemKey::AlbumTitle])),
        album_artist: Some(text(tag, &[ItemKey::AlbumArtist])),
        genre: Some(text(tag, &[ItemKey::Genre])),
        year: Some(
            tag.date()
                .map(|d| d.year.to_string())
                .unwrap_or_else(|| text(tag, &[ItemKey::Year])),
        ),
        track_no: Some(number(tag.track())),
        disc_no: Some(number(tag.disk())),
    })
}

/// A change that can't be written, caught before any file is touched.
pub fn validate(change: &TagFields) -> Result<(), String> {
    let positive = |v: &Option<String>, what: &str| match v.as_deref().map(str::trim) {
        None | Some("") => Ok(()),
        Some(s) => s
            .parse::<u32>()
            .ok()
            .filter(|&n| n > 0)
            .map(|_| ())
            .ok_or(format!("{what} must be a whole number above 0")),
    };
    positive(&change.track_no, "The track number")?;
    positive(&change.disc_no, "The disc number")?;
    match change.year.as_deref().map(str::trim) {
        None | Some("") => Ok(()),
        Some(y) => y
            .parse::<u16>()
            .ok()
            .filter(|y| (1..=9999).contains(y))
            .map(|_| ())
            .ok_or_else(|| "The year must be a number like 1999".to_string()),
    }
}

/// Writes `change` into the file at `path`.
pub fn write_fields(path: &Path, change: &TagFields) -> Result<(), String> {
    validate(change)?;
    let mut file = open(path)?;
    let main = file.primary_tag_type();
    if file.tag(main).is_none() {
        // Start from whatever tag the file has (an MP3 with only ID3v1, say), so nothing
        // the other tag holds is lost.
        let mut tag = file.first_tag().cloned().unwrap_or_else(|| Tag::new(main));
        tag.re_map(main);
        file.insert_tag(tag);
    }
    let tag = file.tag_mut(main).ok_or("no tag to write into")?;
    apply(tag, change);
    tag.save_to_path(path, WriteOptions::default())
        .map_err(|e| e.to_string())
}

fn apply(tag: &mut Tag, change: &TagFields) {
    fn set(tag: &mut Tag, keys: &[ItemKey], value: &Option<String>) {
        let Some(value) = value else { return };
        for key in keys {
            tag.remove_key(*key);
        }
        let value = value.trim();
        if !value.is_empty() {
            tag.insert_text(keys[0], value.to_string());
        }
    }
    set(tag, &[ItemKey::TrackTitle], &change.title);
    // The multi-value form goes too: readers prefer it over the plain one.
    set(
        tag,
        &[ItemKey::TrackArtist, ItemKey::TrackArtists],
        &change.artist,
    );
    set(tag, &[ItemKey::AlbumTitle], &change.album);
    set(tag, &[ItemKey::AlbumArtist], &change.album_artist);
    set(tag, &[ItemKey::Genre], &change.genre);
    if let Some(year) = &change.year {
        tag.remove_date();
        if let Ok(year) = year.trim().parse::<u16>() {
            tag.set_date(lofty::tag::items::Timestamp {
                year,
                ..Default::default()
            });
        }
    }
    let number =
        |tag: &mut Tag, value: &Option<String>, set: fn(&mut Tag, u32), remove: fn(&mut Tag)| {
            if let Some(v) = value {
                match v.trim().parse::<u32>() {
                    Ok(n) if n > 0 => set(tag, n),
                    _ => remove(tag),
                }
            }
        };
    number(tag, &change.track_no, Tag::set_track, Tag::remove_track);
    number(tag, &change.disc_no, Tag::set_disk, Tag::remove_disk);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_years_are_checked_first() {
        assert!(validate(&TagFields::default()).is_ok());
        let ok = TagFields {
            year: Some("1999".into()),
            track_no: Some(" 3 ".into()),
            disc_no: Some(String::new()),
            ..TagFields::default()
        };
        assert!(validate(&ok).is_ok());
        for bad in [
            TagFields {
                track_no: Some("3a".into()),
                ..TagFields::default()
            },
            TagFields {
                disc_no: Some("0".into()),
                ..TagFields::default()
            },
            TagFields {
                year: Some("nineteen".into()),
                ..TagFields::default()
            },
        ] {
            assert!(validate(&bad).is_err(), "{bad:?}");
        }
    }
}
