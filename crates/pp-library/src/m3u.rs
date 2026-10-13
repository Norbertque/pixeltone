//! M3U / M3U8 playlists: reading (with `#EXTINF` hints, relative paths resolved against the
//! playlist's folder) and writing (UTF-8, absolute paths).

use std::path::{Path, PathBuf};

use crate::model::{PlaylistInput, TrackRow};

/// UTF-8 (with or without BOM), or Latin-1 for older `.m3u` files.
fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

/// `%20` → ` ` in `file://` URLs.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The entries of the playlist at `file`. Web addresses are skipped.
pub fn parse(text: &str, base: &Path) -> Vec<PlaylistInput> {
    let mut items = Vec::new();
    let mut hint: Option<(Option<u32>, Option<String>, Option<String>)> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(info) = line.strip_prefix("#EXTINF:") {
            // #EXTINF:<seconds>[ attributes],<artist> - <title>
            let (head, name) = info.split_once(',').unwrap_or((info, ""));
            let seconds = head
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|s| *s > 0.0)
                .map(|s| (s * 1000.0) as u32);
            let (artist, title) = match name.split_once(" - ") {
                Some((a, t)) => (Some(a.trim().to_string()), Some(t.trim().to_string())),
                None => (
                    None,
                    Some(name.trim().to_string()).filter(|t| !t.is_empty()),
                ),
            };
            hint = Some((seconds, artist, title));
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let path = if let Some(rest) = line.strip_prefix("file:///") {
            PathBuf::from(percent_decode(rest))
        } else if line.contains("://") {
            hint = None;
            continue;
        } else {
            let p = PathBuf::from(line);
            if p.is_absolute() || line.starts_with('\\') {
                p
            } else {
                base.join(p)
            }
        };
        let (duration_ms, artist, title) = hint.take().unwrap_or_default();
        items.push(PlaylistInput {
            path: path.to_string_lossy().into_owned(),
            title,
            artist,
            duration_ms,
        });
    }
    items
}

/// Reads a playlist file: its name (the file name) and entries.
pub fn read(file: &Path) -> std::io::Result<(String, Vec<PlaylistInput>)> {
    let text = decode(&std::fs::read(file)?);
    let base = file.parent().unwrap_or(Path::new(""));
    let name = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Imported playlist".into());
    Ok((name, parse(&text, base)))
}

pub fn render(tracks: &[TrackRow]) -> String {
    let mut out = String::from("#EXTM3U\n");
    for t in tracks {
        let seconds = (t.duration_ms + 500) / 1000;
        let name = if t.artist.is_empty() {
            t.title.clone()
        } else {
            format!("{} - {}", t.artist, t.title)
        };
        out.push_str(&format!("#EXTINF:{seconds},{name}\n{}\n", t.path));
    }
    out
}

pub fn write(file: &Path, tracks: &[TrackRow]) -> std::io::Result<()> {
    std::fs::write(file, render(tracks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_extinf_hints_and_resolves_relative_paths() {
        let text = "#EXTM3U\r\n#EXTINF:215,Aurora Lane - Neon Tidepool\r\nAlbum/03.flac\r\n\
                    #EXTINF:-1,Stream\r\nhttp://radio.example/stream\r\n\
                    file:///C:/Music/My%20Song.mp3\r\n# a comment\r\n/abs/x.flac\r\n";
        let items = parse(text, Path::new("/music/lists"));
        assert_eq!(items.len(), 3);
        assert_eq!(
            items[0].path,
            Path::new("/music/lists")
                .join("Album/03.flac")
                .to_string_lossy()
        );
        assert_eq!(items[0].artist.as_deref(), Some("Aurora Lane"));
        assert_eq!(items[0].title.as_deref(), Some("Neon Tidepool"));
        assert_eq!(items[0].duration_ms, Some(215_000));
        assert_eq!(items[1].path, "C:/Music/My Song.mp3");
        assert_eq!(
            items[1].title, None,
            "the skipped stream's hint doesn't leak"
        );
        assert_eq!(items[2].path, "/abs/x.flac");
    }

    #[test]
    fn decodes_latin1_files() {
        assert_eq!(decode(b"Bj\xF6rk.flac"), "Björk.flac");
        assert_eq!(decode("\u{FEFF}é".as_bytes()), "é");
    }

    #[test]
    fn round_trips() {
        let mut t = TrackRow::outside("/m/a.flac", Some("Title"), Some("Artist"), 61_400);
        t.id = 1;
        let text = render(&[t]);
        assert_eq!(text, "#EXTM3U\n#EXTINF:61,Artist - Title\n/m/a.flac\n");
        let back = parse(&text, Path::new("/"));
        assert_eq!(back[0].path, "/m/a.flac");
        assert_eq!(back[0].title.as_deref(), Some("Title"));
    }
}
