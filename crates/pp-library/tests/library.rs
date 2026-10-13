//! Library scanning against a fixture library generated with the fetched `ffmpeg`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use pp_library::{
    AlbumSort, Library, PlayRecord, PlaylistInput, ProgressFn, ScanPhase, ScanProgress, TrackRow,
    TrackSort,
};

fn ffmpeg(args: &[&str]) {
    let bin = std::env::var("PP_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let out = Command::new(bin)
        .args(["-hide_banner", "-v", "error", "-y"])
        .args(args)
        .output()
        .expect("run ffmpeg");
    assert!(
        out.status.success(),
        "ffmpeg {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn png(path: &Path, color: &str) {
    let src = format!("color=c={color}:s=320x320:d=1");
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        &src,
        "-frames:v",
        "1",
        path.to_str().unwrap(),
    ]);
}

struct Tags<'a> {
    title: &'a str,
    artist: &'a str,
    album: &'a str,
    album_artist: Option<&'a str>,
    track: u32,
    disc: Option<u32>,
    year: Option<u32>,
    genre: Option<&'a str>,
}

/// Writes a short tagged tone; `cover` embeds a picture.
fn track(path: &Path, tags: &Tags, cover: Option<&Path>) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let freq = 220 + tags.track * 40;
    let src = format!("sine=f={freq}:r=44100:d=0.2");
    let mut args: Vec<String> = vec!["-f".into(), "lavfi".into(), "-i".into(), src];
    if let Some(c) = cover {
        args.extend([
            "-i".into(),
            c.to_str().unwrap().into(),
            "-map".into(),
            "0:a".into(),
            "-map".into(),
            "1:v".into(),
        ]);
        args.extend([
            "-c:v".into(),
            "png".into(),
            "-disposition:v".into(),
            "attached_pic".into(),
        ]);
    }
    let mut meta = vec![
        format!("title={}", tags.title),
        format!("artist={}", tags.artist),
        format!("album={}", tags.album),
        format!("track={}", tags.track),
    ];
    if let Some(aa) = tags.album_artist {
        meta.push(format!("album_artist={aa}"));
    }
    if let Some(d) = tags.disc {
        meta.push(format!("disc={d}"));
    }
    if let Some(y) = tags.year {
        meta.push(format!("date={y}"));
    }
    if let Some(g) = tags.genre {
        meta.push(format!("genre={g}"));
    }
    for m in meta {
        args.extend(["-metadata".into(), m]);
    }
    args.push(path.to_str().unwrap().into());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    ffmpeg(&refs);
}

fn t<'a>(title: &'a str, artist: &'a str, album: &'a str, n: u32) -> Tags<'a> {
    Tags {
        title,
        artist,
        album,
        album_artist: None,
        track: n,
        disc: None,
        year: None,
        genre: None,
    }
}

/// The fixture library; returns its root.
fn build(root: &Path) {
    let covers = root.join("_covers_src");
    std::fs::create_dir_all(&covers).unwrap();
    let violet = covers.join("violet.png");
    png(&violet, "0x7e57c2");

    // An album with an album artist, an embedded cover and a featured artist.
    let pd = root.join("Aurora Lane/Pixel Dreams");
    for (n, title, artist) in [
        (1, "Neon Tidepool", "Aurora Lane"),
        (2, "Glass Garden", "Aurora Lane feat. Neon Child"),
        (3, "Afterglow", "Aurora Lane"),
    ] {
        let tags = Tags {
            album_artist: Some("Aurora Lane"),
            year: Some(2024),
            genre: Some("Synthpop; Electronic"),
            ..t(title, artist, "Pixel Dreams", n)
        };
        track(
            &pd.join(format!("0{n} {title}.flac")),
            &tags,
            (n == 1).then_some(violet.as_path()),
        );
    }

    // Two discs in sub-folders, one album.
    let dbl = root.join("Kestrel/Double");
    for disc in 1..=2 {
        for n in 1..=2 {
            let title = format!("D{disc}T{n}");
            let tags = Tags {
                album_artist: Some("Kestrel"),
                disc: Some(disc),
                year: Some(2019),
                ..t(&title, "Kestrel", "Double", n)
            };
            track(&dbl.join(format!("CD{disc}/{n:02}.flac")), &tags, None);
        }
    }
    // A cover image file next to the music.
    png(&dbl.join("CD1/cover.png"), "0xff8800");

    // A compilation without album artist: one album, "Various artists".
    let mix = root.join("Compilations/Summer Mix");
    track(
        &mix.join("01.mp3"),
        &t("Sunny", "Sol", "Summer Mix", 1),
        None,
    );
    track(
        &mix.join("02.mp3"),
        &t("Breeze", "Mar", "Summer Mix", 2),
        None,
    );

    // Same title by two album artists: two albums.
    for artist in ["Alpha", "Beta"] {
        let tags = Tags {
            album_artist: Some(artist),
            ..t("Intro", artist, "Greatest Hits", 1)
        };
        track(
            &root.join(format!("{artist}/Greatest Hits/01.flac")),
            &tags,
            None,
        );
    }

    // Diacritics, and a format lofty can't read (Matroska) to exercise the FFmpeg path.
    track(
        &root.join("Misc/bjork.flac"),
        &t("Jóga", "Björk", "Homogenic", 1),
        None,
    );
    track(
        &root.join("Misc/odd.mka"),
        &t("Matroska Song", "Mk Artist", "Mk Album", 1),
        None,
    );

    std::fs::remove_dir_all(covers).unwrap();
}

fn open(dir: &Path) -> Arc<Library> {
    Library::open(dir.join("library.db"), dir.join("art")).unwrap()
}

fn scan(lib: &Library) -> ScanProgress {
    let progress: ProgressFn = Arc::new(|_| {});
    lib.scan_once(&progress).unwrap()
}

struct Fixture {
    _dir: tempfile::TempDir,
    data: PathBuf,
    music: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("Music");
    let data = dir.path().join("data");
    build(&music);
    Fixture {
        data,
        music,
        _dir: dir,
    }
}

#[test]
fn scans_groups_and_searches() {
    let f = fixture();
    let lib = open(&f.data);
    lib.add_folder(&f.music).unwrap();
    let p = scan(&lib);
    assert_eq!(p.phase, ScanPhase::Done);
    assert_eq!(p.errors, 0);
    assert_eq!(p.added, 13);

    let o = lib.overview().unwrap();
    assert_eq!(o.track_count, 13);
    // Pixel Dreams, Double, Summer Mix, Greatest Hits ×2, Homogenic, Mk Album.
    assert_eq!(o.album_count, 7);

    let albums = lib.albums(AlbumSort::Title, 0, 100).unwrap();
    let pd = albums.iter().find(|a| a.title == "Pixel Dreams").unwrap();
    assert_eq!(pd.artist, "Aurora Lane");
    assert_eq!(pd.year, Some(2024));
    assert_eq!(pd.track_count, 3);
    assert!(pd.cover.is_some(), "embedded cover from track 1");
    let cover = pd.cover.as_ref().unwrap();
    assert!(lib.art().existing(cover, 300).is_some());

    let mix = albums.iter().find(|a| a.title == "Summer Mix").unwrap();
    assert_eq!(mix.artist, "Various artists");
    assert_eq!(mix.track_count, 2);

    assert_eq!(
        albums.iter().filter(|a| a.title == "Greatest Hits").count(),
        2
    );

    let double = albums.iter().find(|a| a.title == "Double").unwrap();
    let detail = lib.album(double.id).unwrap().unwrap();
    let order: Vec<&str> = detail.tracks.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(order, ["D1T1", "D1T2", "D2T1", "D2T2"]);
    assert!(detail.tracks[0].cover.is_some(), "cover.png next to disc 1");

    // Rows by id keep the requested order and skip unknown ids.
    let ids: Vec<u32> = detail
        .tracks
        .iter()
        .rev()
        .map(|t| t.id)
        .chain([999_999])
        .collect();
    let rows = lib.tracks_by_id(&ids).unwrap();
    let titles: Vec<&str> = rows.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(titles, ["D2T2", "D2T1", "D1T2", "D1T1"]);

    // The featured artist became an artist of their own.
    let artists = lib.artists(0, 100).unwrap();
    let names: Vec<&str> = artists.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"Neon Child"), "{names:?}");
    let aurora = artists.iter().find(|a| a.name == "Aurora Lane").unwrap();
    assert_eq!(aurora.track_count, 3);
    let aurora = lib.artist(aurora.id).unwrap().unwrap();
    assert_eq!(aurora.albums.len(), 1);

    let genres = lib.genres().unwrap();
    let gnames: Vec<&str> = genres.iter().map(|g| g.name.as_str()).collect();
    assert!(
        gnames.contains(&"Synthpop") && gnames.contains(&"Electronic"),
        "{gnames:?}"
    );

    // Diacritics-insensitive, prefix search.
    let r = lib.search("bjor", 20).unwrap();
    assert_eq!(r.tracks.len(), 1);
    assert_eq!(r.artists.len(), 1);
    let r = lib.search("pixel dre", 20).unwrap();
    assert_eq!(r.albums.len(), 1);
    assert_eq!(r.tracks.len(), 3);

    // The Matroska file was read through FFmpeg.
    let mk = lib.search("matroska", 5).unwrap();
    assert_eq!(mk.tracks.len(), 1);
    assert_eq!(mk.tracks[0].artist, "Mk Artist");

    // Sorting and paging.
    let first = lib.tracks(TrackSort::Title, 0, 3).unwrap();
    assert_eq!(first[0].title, "Afterglow");
    assert_eq!(lib.tracks(TrackSort::Title, 12, 10).unwrap().len(), 1);
}

#[test]
fn rescans_incrementally() {
    let f = fixture();
    let lib = open(&f.data);
    lib.add_folder(&f.music).unwrap();
    scan(&lib);

    let p = scan(&lib);
    assert_eq!(
        (p.total, p.added, p.updated, p.removed),
        (0, 0, 0, 0),
        "nothing changed"
    );

    // Re-tag one file: it is re-read; delete another: it disappears with its album.
    let changed = f.music.join("Misc/bjork.flac");
    track(
        &changed,
        &t("Jóga (Remastered)", "Björk", "Homogenic", 1),
        None,
    );
    std::fs::remove_file(f.music.join("Misc/odd.mka")).unwrap();
    let p = scan(&lib);
    assert_eq!((p.total, p.added, p.updated, p.removed), (1, 0, 1, 1));
    assert_eq!(lib.search("remastered", 5).unwrap().tracks.len(), 1);
    assert!(
        lib.albums(AlbumSort::Title, 0, 100)
            .unwrap()
            .iter()
            .all(|a| a.title != "Mk Album")
    );
    assert_eq!(lib.overview().unwrap().track_count, 12);
}

#[test]
fn an_unreachable_folder_keeps_its_tracks() {
    let f = fixture();
    let lib = open(&f.data);
    lib.add_folder(&f.music).unwrap();
    scan(&lib);
    let moved = f.music.with_file_name("Music-offline");
    std::fs::rename(&f.music, &moved).unwrap();
    let p = scan(&lib);
    assert_eq!(p.removed, 0);
    assert_eq!(lib.overview().unwrap().track_count, 13);
}

#[test]
fn browses_folders_and_removes_them() {
    let f = fixture();
    let lib = open(&f.data);
    lib.add_folder(&f.music).unwrap();
    scan(&lib);

    let top = lib.folder(None).unwrap();
    assert_eq!(top.folders.len(), 1);
    let root = lib.folder(Some(&top.folders[0].path)).unwrap();
    let names: Vec<&str> = root.folders.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Alpha",
            "Aurora Lane",
            "Beta",
            "Compilations",
            "Kestrel",
            "Misc"
        ]
    );
    assert_eq!(root.parent, None);
    let kestrel = root.folders.iter().find(|e| e.name == "Kestrel").unwrap();
    assert_eq!(kestrel.track_count, 4);
    let misc = lib
        .folder(Some(
            &root.folders.iter().find(|e| e.name == "Misc").unwrap().path,
        ))
        .unwrap();
    assert_eq!(misc.tracks.len(), 2);
    assert!(misc.parent.is_some());

    let id = lib.folders().unwrap()[0].id;
    lib.remove_folder(id).unwrap();
    let o = lib.overview().unwrap();
    assert_eq!((o.track_count, o.album_count, o.artist_count), (0, 0, 0));
    assert!(lib.search("aurora", 5).unwrap().tracks.is_empty());
}

#[test]
fn delimiter_changes_take_effect_on_rescan() {
    let f = fixture();
    let lib = open(&f.data);
    lib.add_folder(&f.music).unwrap();
    scan(&lib);
    let names = |lib: &Library| -> Vec<String> {
        lib.artists(0, 100)
            .unwrap()
            .into_iter()
            .map(|a| a.name)
            .collect()
    };
    assert!(names(&lib).contains(&"Neon Child".to_string()));

    lib.set_delimiters(vec![";".into()]).unwrap();
    let p = scan(&lib);
    assert_eq!(p.updated, 13, "every file is re-read");
    let after = names(&lib);
    assert!(
        after.contains(&"Aurora Lane feat. Neon Child".to_string()),
        "{after:?}"
    );
}

/// `cargo test -p pp-library --release -- --ignored --nocapture large_library`
fn titles(tracks: &[TrackRow]) -> Vec<&str> {
    tracks.iter().map(|t| t.title.as_str()).collect()
}

fn album_tracks(lib: &Library, title: &str) -> Vec<TrackRow> {
    let album = lib
        .albums(AlbumSort::Title, 0, 100)
        .unwrap()
        .into_iter()
        .find(|a| a.title == title)
        .unwrap();
    lib.album(album.id).unwrap().unwrap().tracks
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn playlists_keep_their_order_and_survive_rescans() {
    let f = fixture();
    let lib = open(&f.data);
    lib.add_folder(&f.music).unwrap();
    scan(&lib);
    let pd = album_tracks(&lib, "Pixel Dreams");
    let ids: Vec<u32> = pd.iter().map(|t| t.id).collect();

    let id = lib.create_playlist("  Road trip ", &[]).unwrap();
    assert!(
        lib.create_playlist("   ", &[]).is_err(),
        "a name is required"
    );
    lib.playlist_add_tracks(id, &ids).unwrap();
    lib.playlist_add(
        id,
        &[PlaylistInput {
            path: "/elsewhere/song.mp3".into(),
            title: Some("Elsewhere".into()),
            artist: Some("Someone".into()),
            duration_ms: Some(1000),
        }],
    )
    .unwrap();
    let d = lib.playlist(id).unwrap().unwrap();
    assert_eq!(d.playlist.name, "Road trip");
    assert_eq!(d.playlist.track_count, 4);
    assert_eq!(
        titles(&d.tracks),
        ["Neon Tidepool", "Glass Garden", "Afterglow", "Elsewhere"]
    );
    assert_eq!(d.tracks[3].id, 0, "outside the library");
    assert_eq!(d.tracks[3].artist, "Someone");
    assert_eq!(d.playlist.covers.len(), 1);

    lib.playlist_move(id, 3, 0).unwrap();
    lib.playlist_remove(id, &[2]).unwrap();
    let order = |lib: &Library| {
        titles(&lib.playlist(id).unwrap().unwrap().tracks)
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(order(&lib), ["Elsewhere", "Neon Tidepool", "Afterglow"]);
    scan(&lib);
    assert_eq!(
        order(&lib),
        ["Elsewhere", "Neon Tidepool", "Afterglow"],
        "a rescan leaves it alone"
    );

    lib.rename_playlist(id, "Trip").unwrap();
    assert_eq!(lib.playlists().unwrap()[0].name, "Trip");

    let file = f.data.join("Trip.m3u8");
    lib.export_m3u(id, &file).unwrap();
    let copy = lib.import_m3u(&file).unwrap();
    let copied = lib.playlist(copy).unwrap().unwrap();
    assert_eq!(copied.playlist.name, "Trip");
    let paths = |t: &[TrackRow]| t.iter().map(|t| t.path.clone()).collect::<Vec<_>>();
    assert_eq!(
        paths(&copied.tracks),
        paths(&lib.playlist(id).unwrap().unwrap().tracks)
    );
    assert_eq!(
        copied.tracks[0].title, "Elsewhere",
        "hints come back from #EXTINF"
    );

    lib.delete_playlist(id).unwrap();
    assert!(lib.playlist(id).unwrap().is_none());
    assert_eq!(lib.playlists().unwrap().len(), 1);
}

#[test]
fn artist_pictures_found_online_replace_album_covers() {
    let fx = fixture();
    let lib = open(&fx.data);
    lib.add_folder(&fx.music).unwrap();
    scan(&lib);

    let todo = lib.artists_without_images(now(), 100).unwrap();
    // Most tracks first: Aurora Lane has three (plus an album artist role), Kestrel four.
    assert_eq!(todo[0].0, "Kestrel");
    assert!(
        todo.iter()
            .any(|(name, key)| name == "Björk" && key == "björk")
    );

    let kestrel = |lib: &Library| {
        lib.artists(0, 100)
            .unwrap()
            .into_iter()
            .find(|a| a.name == "Kestrel")
            .unwrap()
    };
    let album_cover = kestrel(&lib).cover.clone();
    assert!(album_cover.is_some(), "the folder image stands in");

    let picture_file = fx.data.join("kestrel.png");
    png(&picture_file, "0x00aa55");
    let picture = std::fs::read(picture_file).unwrap();
    lib.set_artist_image("kestrel", Some(&picture)).unwrap();
    let pictured = kestrel(&lib).cover.clone().unwrap();
    assert_ne!(Some(&pictured), album_cover.as_ref());
    assert!(
        lib.art().existing(&pictured, 300).is_some(),
        "thumbnails written"
    );
    assert!(lib.artist(kestrel(&lib).id).unwrap().unwrap().artist.cover == Some(pictured));

    // Nothing found: not asked again until the retry time passes.
    lib.set_artist_image("björk", None).unwrap();
    let todo = lib.artists_without_images(0, 100).unwrap();
    assert!(
        !todo
            .iter()
            .any(|(_, key)| key == "kestrel" || key == "björk")
    );
    assert!(
        lib.artists_without_images(now() + 1, 100)
            .unwrap()
            .iter()
            .any(|(_, key)| key == "björk")
    );

    // Rescans keep them; forgetting brings the album covers back.
    scan(&lib);
    assert_ne!(kestrel(&lib).cover, album_cover);
    lib.forget_artist_images().unwrap();
    assert_eq!(kestrel(&lib).cover, album_cover);
}

#[test]
fn favourites_show_up_in_track_lists() {
    let f = fixture();
    let lib = open(&f.data);
    lib.add_folder(&f.music).unwrap();
    scan(&lib);
    let after = album_tracks(&lib, "Pixel Dreams")[2].clone();
    assert_eq!(after.title, "Afterglow");
    assert!(!after.favorite);

    lib.set_favorite(&after.path, true).unwrap();
    lib.set_favorite(&after.path, true).unwrap();
    let tracks = album_tracks(&lib, "Pixel Dreams");
    assert_eq!(
        tracks.iter().map(|t| t.favorite).collect::<Vec<_>>(),
        [false, false, true]
    );
    assert_eq!(titles(&lib.favorites().unwrap()), ["Afterglow"]);
    assert!(lib.is_favorite(&after.path).unwrap());

    lib.set_favorite(&after.path, false).unwrap();
    assert!(lib.favorites().unwrap().is_empty());
}

#[test]
fn history_feeds_stats_recents_and_the_daily_mix() {
    let f = fixture();
    let lib = open(&f.data);
    lib.add_folder(&f.music).unwrap();
    scan(&lib);
    let pd = album_tracks(&lib, "Pixel Dreams");
    let intro = album_tracks(&lib, "Greatest Hits")[0].clone();
    let t = now();
    let play = |track: &TrackRow, ago: i64, ms: u32| {
        lib.record_play(&PlayRecord {
            path: track.path.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            played_at: t - ago,
            ms_played: ms,
        })
        .unwrap();
    };
    play(&pd[0], 7200, 1000);
    play(&pd[0], 3600, 1000);
    play(&pd[0], 3 * 86_400, 1000);
    play(&intro, 100, 500);
    play(&pd[1], 40 * 86_400, 800);
    play(
        &TrackRow::outside("/gone/old.flac", Some("Gone"), Some("Nobody"), 0),
        50,
        300,
    );

    let month = lib.stats(Some(30)).unwrap();
    assert_eq!(month.plays, 5);
    assert_eq!(month.total_ms, 3800.0);
    assert_eq!(month.distinct_tracks, 3);
    assert_eq!(month.top_tracks[0].track.title, "Neon Tidepool");
    assert_eq!(month.top_tracks[0].plays, 3);
    assert!(
        month
            .top_tracks
            .iter()
            .any(|t| t.track.id == 0 && t.track.title == "Gone")
    );
    assert_eq!(month.top_artists[0].artist.name, "Aurora Lane");
    assert_eq!(month.top_albums[0].album.title, "Pixel Dreams");
    assert_eq!(month.per_hour.len(), 24);
    assert_eq!(month.per_hour.iter().sum::<f64>(), 3800.0);
    assert_eq!(month.per_day.iter().map(|d| d.ms).sum::<f64>(), 3800.0);
    assert_eq!(lib.stats(None).unwrap().plays, 6);

    let home = lib.home().unwrap();
    assert_eq!(
        titles(&home.recent_tracks),
        ["Intro", "Neon Tidepool", "Glass Garden"]
    );
    assert_eq!(home.recent_albums[0].title, "Greatest Hits");
    assert_eq!(home.top_artists[0].artist.name, "Aurora Lane");

    let mix = lib.daily_mix().unwrap();
    assert_eq!(mix.len(), 13, "the whole (small) library");
    let mut ids: Vec<u32> = mix.iter().map(|t| t.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 13, "no repeats");
    let again: Vec<u32> = lib.daily_mix().unwrap().iter().map(|t| t.id).collect();
    assert_eq!(
        again,
        mix.iter().map(|t| t.id).collect::<Vec<_>>(),
        "stable for the day"
    );
}

#[test]
#[ignore = "slow: builds a 10k-track library"]
fn large_library() {
    use std::time::Instant;
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("Music");
    // One encoded file copied many times with distinct tags is far quicker than encoding 10k.
    let seed = dir.path().join("seed.flac");
    track(&seed, &t("x", "x", "x", 1), None);
    let bytes = std::fs::read(&seed).unwrap();
    for i in 0..10_000u32 {
        let artist = i % 400;
        let album = i / 12;
        let path = music.join(format!("Artist {artist}/Album {album}/{i:05}.flac"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &bytes).unwrap();
    }
    let lib = Library::open(dir.path().join("lib.db"), dir.path().join("art")).unwrap();
    lib.add_folder(&music).unwrap();

    let t0 = Instant::now();
    let p = scan(&lib);
    println!("full scan of {} files: {:?}", p.total, t0.elapsed());
    let t1 = Instant::now();
    let p = scan(&lib);
    println!(
        "rescan with no changes ({} read): {:?}",
        p.total,
        t1.elapsed()
    );
    let t2 = Instant::now();
    let rows = lib.tracks(TrackSort::Artist, 5_000, 100).unwrap();
    let hits = lib.search("x", 50).unwrap();
    println!(
        "page query + search: {:?} ({} rows, {} hits)",
        t2.elapsed(),
        rows.len(),
        hits.tracks.len()
    );
}
