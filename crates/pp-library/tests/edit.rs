//! The tag editor on real files: what is written reads back, the rest of the tag survives,
//! and a rescan shows the change.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use lofty::prelude::*;
use pp_library::edit::{TagFields, read_fields, write_fields};
use pp_library::{Library, ProgressFn, TrackSort};

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

/// A short tagged tone with an embedded cover, in the format `path`'s extension implies.
fn tagged(path: &Path, codec: &[&str]) {
    let cover = path.with_extension("png");
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "color=c=red:s=64x64:d=1",
        "-frames:v",
        "1",
        cover.to_str().unwrap(),
    ]);
    let mut args = vec![
        "-f",
        "lavfi",
        "-i",
        "sine=f=440:r=44100:d=0.2",
        "-i",
        cover.to_str().unwrap(),
        "-map",
        "0:a",
        "-map",
        "1:v",
        "-disposition:v",
        "attached_pic",
        "-metadata",
        "title=Old Title",
        "-metadata",
        "artist=Old Artist",
        "-metadata",
        "album=Old Album",
        "-metadata",
        "date=2001",
        "-metadata",
        "track=3",
        "-metadata",
        "comment=keep me",
    ];
    args.extend_from_slice(codec);
    args.push(path.to_str().unwrap());
    ffmpeg(&args);
    std::fs::remove_file(cover).unwrap();
}

fn change() -> TagFields {
    TagFields {
        title: Some("New Title".into()),
        artist: Some("New Artist".into()),
        album: None, // left alone
        album_artist: Some("Various Artists".into()),
        genre: Some("Breakcore".into()),
        year: Some("2019".into()),
        track_no: Some(String::new()), // removed
        disc_no: Some("2".into()),
    }
}

#[test]
fn edits_read_back_in_every_common_format() {
    let dir = tempfile::tempdir().unwrap();
    let files = [
        ("a.flac", vec!["-c:a", "flac"]),
        ("b.mp3", vec!["-c:a", "libmp3lame", "-id3v2_version", "3"]),
        ("c.m4a", vec!["-c:a", "alac", "-c:v", "png"]),
    ];
    for (name, codec) in &files {
        let path = dir.path().join(name);
        tagged(&path, codec);
        let before = read_fields(&path).unwrap();
        assert_eq!(before.title.as_deref(), Some("Old Title"), "{name}");
        assert_eq!(before.track_no.as_deref(), Some("3"), "{name}");
        let untouched = |path: &Path| {
            let file = lofty::read_from_path(path).unwrap();
            let tag = file.primary_tag().unwrap();
            let mut items: Vec<String> = tag
                .items()
                .filter(|i| !EDITED.contains(&i.key()))
                .map(|i| format!("{:?}={:?}", i.key(), i.value()))
                .collect();
            items.sort();
            (items, tag.pictures().len())
        };
        let kept = untouched(&path);
        assert!(
            !kept.0.is_empty(),
            "{name}: something besides the edited fields"
        );

        write_fields(&path, &change()).unwrap();
        let after = read_fields(&path).unwrap();
        assert_eq!(
            after,
            TagFields {
                album: Some("Old Album".into()),
                track_no: Some(String::new()),
                ..change()
            },
            "{name}"
        );
        // The cover and the fields the editor doesn't show are still there.
        assert_eq!(untouched(&path), kept, "{name}");
        assert_eq!(kept.1, 1, "{name}: one cover");
    }
}

const EDITED: [ItemKey; 10] = [
    ItemKey::TrackTitle,
    ItemKey::TrackArtist,
    ItemKey::TrackArtists,
    ItemKey::AlbumTitle,
    ItemKey::AlbumArtist,
    ItemKey::Genre,
    ItemKey::Year,
    ItemKey::RecordingDate,
    ItemKey::TrackNumber,
    ItemKey::DiscNumber,
];

#[test]
fn bad_values_leave_the_file_alone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.flac");
    tagged(&path, &["-c:a", "flac"]);
    let before = std::fs::read(&path).unwrap();
    let err = write_fields(
        &path,
        &TagFields {
            title: Some("Fine".into()),
            year: Some("soon".into()),
            ..TagFields::default()
        },
    )
    .unwrap_err();
    assert!(err.contains("year"), "{err}");
    assert!(std::fs::read(&path).unwrap() == before);
}

#[test]
fn files_lofty_cant_write_say_so() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.dsf");
    pp_decode::testing::write_dsf(&path, 2_822_400, &[vec![0x69; 4096], vec![0x69; 4096]]).unwrap();
    let err = write_fields(&path, &change()).unwrap_err();
    assert!(err.contains("can't be edited"), "{err}");
}

#[test]
fn a_rescan_shows_the_edit() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("Music");
    std::fs::create_dir_all(&music).unwrap();
    let path = music.join("a.flac");
    tagged(&path, &["-c:a", "flac"]);
    let lib = Library::open(dir.path().join("library.db"), dir.path().join("art")).unwrap();
    lib.add_folder(&music).unwrap();
    let progress: ProgressFn = Arc::new(|_| {});
    lib.scan_once(&progress).unwrap();

    // Same second as the scan: the size changes, so the rescan still notices.
    write_fields(&path, &change()).unwrap();
    lib.scan_once(&progress).unwrap();
    let tracks = lib.tracks(TrackSort::Title, 0, 10).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "New Title");
    assert_eq!(tracks[0].artist, "New Artist");
    assert_eq!(tracks[0].year, Some(2019));
    assert_eq!(tracks[0].disc_no, Some(2));
}
