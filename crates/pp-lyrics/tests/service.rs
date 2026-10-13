//! The lyrics service against real files and a stand-in LRCLIB server.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use pp_lyrics::{LyricsQuery, LyricsService, LyricsSource};

fn ffmpeg(args: &[&str]) {
    let bin = std::env::var("PP_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let out = Command::new(bin)
        .args(["-hide_banner", "-v", "error", "-y"])
        .args(args)
        .output()
        .expect("run ffmpeg");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A short FLAC, optionally with a LYRICS tag.
fn flac(path: &Path, lyrics: Option<&str>) {
    let meta = lyrics.map(|l| format!("lyrics={l}"));
    let mut args = vec!["-f", "lavfi", "-i", "sine=d=0.2"];
    if let Some(m) = &meta {
        args.extend(["-metadata", m]);
    }
    let p = path.to_str().unwrap().to_string();
    args.push(&p);
    ffmpeg(&args);
}

/// Serves `/api/get` and `/api/search` from canned answers; records request paths.
fn server(get: Option<&'static str>, search: &'static str) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut first = String::new();
            reader.read_line(&mut first).unwrap();
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 2 {
                line.clear();
            }
            let path = first.split_whitespace().nth(1).unwrap_or("").to_string();
            log.lock().unwrap().push(path.clone());
            let (status, body) = if path.starts_with("/api/get") {
                match get {
                    Some(body) => ("200 OK", body),
                    None => ("404 Not Found", "{\"code\":404}"),
                }
            } else {
                ("200 OK", search)
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (addr, seen)
}

fn query(path: PathBuf) -> LyricsQuery {
    LyricsQuery {
        path,
        title: "Neon Tidepool".into(),
        artist: "Aurora Lane".into(),
        album: "Pixel Dreams".into(),
        duration_ms: Some(200_000),
    }
}

#[test]
fn local_sources_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let song = dir.path().join("song.flac");
    flac(&song, Some("Embedded line one\nline two"));
    let svc = LyricsService::with_endpoint(
        dir.path().join("saved"),
        dir.path().join("cache"),
        "http://127.0.0.1:9",
    );
    svc.set_online(false);

    let l = svc.local(&query(song.clone())).expect("embedded");
    assert_eq!(l.source, LyricsSource::Embedded);
    assert!(!l.synced);
    assert_eq!(l.lines[0].text, "Embedded line one");

    std::fs::write(song.with_extension("lrc"), "[00:01.00]From the file").unwrap();
    let l = svc.local(&query(song.clone())).unwrap();
    assert_eq!(l.source, LyricsSource::File);
    assert!(l.synced);
    assert_eq!(l.lines[0].time_ms, Some(1000));
}

#[test]
fn online_lyrics_are_cached_and_misses_remembered() {
    let dir = tempfile::tempdir().unwrap();
    let song = dir.path().join("song.flac");
    flac(&song, None);
    let (addr, seen) = server(
        Some(
            r#"{"duration":200.0,"instrumental":false,"plainLyrics":"Hello","syncedLyrics":"[00:02.50]Hello"}"#,
        ),
        "[]",
    );
    let svc =
        LyricsService::with_endpoint(dir.path().join("saved"), dir.path().join("cache"), &addr);
    let l = svc.find(&query(song.clone())).expect("from LRCLIB");
    assert_eq!(l.source, LyricsSource::Online);
    assert!(l.synced);
    assert_eq!(l.lines[0].time_ms, Some(2500));
    let first = seen.lock().unwrap()[0].clone();
    assert!(first.starts_with("/api/get?track_name=Neon"), "{first}");
    assert!(
        first.contains("album_name=Pixel") && first.contains("duration=200"),
        "{first}"
    );
    svc.find(&query(song.clone())).unwrap();
    assert_eq!(
        seen.lock().unwrap().len(),
        1,
        "the second time comes from the cache"
    );

    // Not found anywhere: asked once, then remembered.
    let (addr, seen) = server(None, "[]");
    let svc =
        LyricsService::with_endpoint(dir.path().join("saved2"), dir.path().join("cache2"), &addr);
    assert!(svc.find(&query(song.clone())).is_none());
    assert_eq!(seen.lock().unwrap().len(), 2, "get, then search");
    assert!(svc.find(&query(song.clone())).is_none());
    assert_eq!(seen.lock().unwrap().len(), 2);

    // Offline mode never asks.
    let svc = LyricsService::with_endpoint(dir.path().join("s3"), dir.path().join("c3"), &addr);
    svc.set_online(false);
    assert!(svc.find(&query(song)).is_none());
    assert_eq!(seen.lock().unwrap().len(), 2);
}

#[test]
fn search_fallback_and_instrumentals() {
    let dir = tempfile::tempdir().unwrap();
    let song = dir.path().join("song.flac");
    flac(&song, None);
    let (addr, _) = server(
        None,
        r#"[{"duration":250.0,"syncedLyrics":"[00:01.00]wrong"},{"duration":201.0,"instrumental":true}]"#,
    );
    let svc =
        LyricsService::with_endpoint(dir.path().join("saved"), dir.path().join("cache"), &addr);
    let l = svc.find(&query(song)).unwrap();
    assert!(l.instrumental);
    assert!(l.lines.is_empty());
}

#[test]
fn saving_writes_next_to_the_file_or_inside_the_app() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    std::fs::create_dir_all(&music).unwrap();
    let song = music.join("song.flac");
    flac(&song, None);
    let svc = LyricsService::with_endpoint(
        dir.path().join("saved"),
        dir.path().join("cache"),
        "http://127.0.0.1:9",
    );
    svc.set_online(false);
    assert!(svc.save(&query(song.clone()), "  \n").is_err());

    let l = svc
        .save(&query(song.clone()), "[00:03.00]Mine\r\n[00:04.00]Too")
        .unwrap();
    assert_eq!(l.source, LyricsSource::File);
    assert_eq!(
        std::fs::read_to_string(song.with_extension("lrc")).unwrap(),
        "[00:03.00]Mine\n[00:04.00]Too"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let locked = dir.path().join("locked");
        std::fs::create_dir_all(&locked).unwrap();
        let other = locked.join("other.flac");
        flac(&other, None);
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
        let l = svc.save(&query(other.clone()), "Plain words").unwrap();
        assert_eq!(l.source, LyricsSource::Saved);
        assert_eq!(
            svc.local(&query(other)).unwrap().source,
            LyricsSource::Saved
        );
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
