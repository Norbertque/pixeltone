//! Reading tags and stream properties: `lofty` for the formats it knows (fast, multi-value
//! aware), FFmpeg through `pp-decode` for everything else (DSF/DFF, WMA, TAK, …).

use std::path::Path;

use lofty::file::FileType;
use lofty::picture::PictureType;
use lofty::prelude::*;

use crate::artists::{split_artists, split_genres};

/// Everything the library stores about one file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScannedTrack {
    pub title: String,
    /// Artist as tagged, for display.
    pub artist: String,
    /// Individual artists after splitting.
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    /// Individual album artists after splitting.
    pub album_artists: Vec<String>,
    pub genres: Vec<String>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub duration_ms: u64,
    pub codec: String,
    pub sample_rate: u32,
    /// Bits per sample of lossless material; 0 when not meaningful (lossy, DSD).
    pub bits: u32,
    pub channels: u32,
    pub lossless: bool,
    pub dsd: bool,
    /// Embedded front cover (or first picture).
    pub cover: Option<Vec<u8>>,
}

fn non_empty(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Parses "3", "3/12" or "03" into 3.
fn leading_number(s: &str) -> Option<u32> {
    let digits: String = s.trim().chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok().filter(|&n| n > 0)
}

fn year_from(s: &str) -> Option<i32> {
    let digits: String = s
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .take(4)
        .collect();
    (digits.len() == 4).then(|| digits.parse().ok()).flatten()
}

pub fn read_track(path: &Path, delimiters: &[String]) -> Result<ScannedTrack, String> {
    match lofty::read_from_path(path) {
        Ok(file) => Ok(from_lofty(path, &file, delimiters)),
        Err(lofty_error) => {
            from_ffmpeg(path, delimiters).map_err(|e| format!("{lofty_error}; FFmpeg: {e}"))
        }
    }
}

fn from_lofty(path: &Path, file: &lofty::file::TaggedFile, delimiters: &[String]) -> ScannedTrack {
    let props = file.properties();
    let bits = u32::from(props.bit_depth().unwrap_or(0));
    let (codec, lossless) = match file.file_type() {
        FileType::Aac => ("aac", false),
        FileType::Aiff => ("pcm", true),
        FileType::Ape => ("ape", true),
        FileType::Flac => ("flac", true),
        FileType::Mpeg => ("mp3", false),
        // ALAC reports a bit depth, AAC doesn't.
        FileType::Mp4 if bits > 0 => ("alac", true),
        FileType::Mp4 => ("aac", false),
        FileType::Mpc => ("musepack", false),
        FileType::Opus => ("opus", false),
        FileType::Vorbis => ("vorbis", false),
        FileType::Speex => ("speex", false),
        FileType::Wav => ("pcm", true),
        FileType::WavPack => ("wavpack", true),
        _ => ("unknown", false),
    };

    let mut track = ScannedTrack {
        duration_ms: props.duration().as_millis() as u64,
        codec: codec.to_string(),
        sample_rate: props.sample_rate().unwrap_or(0),
        bits: if lossless { bits } else { 0 },
        channels: u32::from(props.channels().unwrap_or(2)),
        lossless,
        ..ScannedTrack::default()
    };

    if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
        let mut artist_values: Vec<String> = tag
            .get_strings(ItemKey::TrackArtists)
            .map(str::to_string)
            .collect();
        if artist_values.is_empty() {
            artist_values = tag
                .get_strings(ItemKey::TrackArtist)
                .map(str::to_string)
                .collect();
        }
        track.artist = artist_values.join("; ");
        track.artists = split_artists(&artist_values, delimiters);
        track.title = non_empty(tag.get_string(ItemKey::TrackTitle)).unwrap_or_default();
        track.album = non_empty(tag.get_string(ItemKey::AlbumTitle));
        track.album_artist = non_empty(tag.get_string(ItemKey::AlbumArtist));
        track.album_artists = track
            .album_artist
            .as_deref()
            .map(|aa| split_artists(&[aa], delimiters))
            .unwrap_or_default();
        let genres: Vec<String> = tag
            .get_strings(ItemKey::Genre)
            .map(str::to_string)
            .collect();
        track.genres = split_genres(&genres);
        track.track_no = tag.track().filter(|&n| n > 0).or_else(|| {
            tag.get_string(ItemKey::TrackNumber)
                .and_then(leading_number)
        });
        track.disc_no = tag
            .disk()
            .filter(|&n| n > 0)
            .or_else(|| tag.get_string(ItemKey::DiscNumber).and_then(leading_number));
        track.year = tag
            .date()
            .map(|d| i32::from(d.year))
            .filter(|&y| y > 0)
            .or_else(|| tag.get_string(ItemKey::Year).and_then(year_from));
        let pictures = tag.pictures();
        track.cover = pictures
            .iter()
            .find(|p| p.pic_type() == PictureType::CoverFront)
            .or_else(|| pictures.first())
            .map(|p| p.data().to_vec());
    }
    if track.title.is_empty() {
        track.title = file_stem(path);
    }
    track
}

fn from_ffmpeg(path: &Path, delimiters: &[String]) -> Result<ScannedTrack, String> {
    let decoder = pp_decode::Decoder::open(path).map_err(|e| e.to_string())?;
    let info = decoder.info();
    let tags = decoder.tags();
    let artist = tags.artist.clone().unwrap_or_default();
    Ok(ScannedTrack {
        title: tags.title.clone().unwrap_or_else(|| file_stem(path)),
        artists: if artist.is_empty() {
            Vec::new()
        } else {
            split_artists(&[artist.as_str()], delimiters)
        },
        artist,
        album: tags.album.clone(),
        album_artist: tags.album_artist.clone(),
        album_artists: tags
            .album_artist
            .as_deref()
            .map(|aa| split_artists(&[aa], delimiters))
            .unwrap_or_default(),
        genres: tags
            .genre
            .as_deref()
            .map(|g| split_genres(&[g]))
            .unwrap_or_default(),
        track_no: tags.track.as_deref().and_then(leading_number),
        disc_no: tags.disc.as_deref().and_then(leading_number),
        year: tags.date.as_deref().and_then(year_from),
        duration_ms: info.duration_ms.unwrap_or(0),
        codec: info.codec.clone(),
        sample_rate: if info.dsd {
            info.source.sample_rate * 8
        } else {
            info.source.sample_rate
        },
        bits: if info.lossless && !info.dsd {
            u32::from(info.source.bits)
        } else {
            0
        },
        channels: u32::from(info.source.channels),
        lossless: info.lossless,
        dsd: info.dsd,
        cover: decoder.cover_art().map(|c| c.data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_parsing() {
        assert_eq!(leading_number("3/12"), Some(3));
        assert_eq!(leading_number("07"), Some(7));
        assert_eq!(leading_number("0"), None);
        assert_eq!(leading_number("x"), None);
        assert_eq!(year_from("1999-04-01"), Some(1999));
        assert_eq!(year_from("99"), None);
    }
}
