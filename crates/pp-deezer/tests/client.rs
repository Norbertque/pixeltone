//! The client against a stand-in Deezer server.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use pp_deezer::{Clues, Deezer, DeezerError};

/// Answers `/search/album` and `/search/track` with the given bodies and `/pic.jpg` with a
/// few bytes; records the request paths.
fn server(album: &'static str, track: &'static str) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
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
            let (status, kind, body): (&str, &str, Vec<u8>) = if path.starts_with("/search/album") {
                ("200 OK", "application/json", album.as_bytes().to_vec())
            } else if path.starts_with("/search/track") {
                ("200 OK", "application/json", track.as_bytes().to_vec())
            } else if path == "/pic.jpg" {
                (
                    "200 OK",
                    "image/jpeg",
                    vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3],
                )
            } else {
                ("404 Not Found", "text/plain", b"nope".to_vec())
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(&body);
        }
    });
    (base, seen)
}

fn clues(albums: &[&str], tracks: &[&str]) -> Clues {
    Clues {
        albums: albums.iter().map(|s| s.to_string()).collect(),
        tracks: tracks.iter().map(|s| s.to_string()).collect(),
    }
}

const NO_HITS: &str = r#"{"data":[],"total":0}"#;

#[test]
fn the_artist_with_our_album_wins_over_a_better_known_namesake() {
    let (base, seen) = server(
        r#"{"data":[
            {"title":"Greatest Hits","artist":{"id":1,"name":"Sally","picture_xl":"http://x/famous.jpg"}},
            {"title":"underdogs","artist":{"id":2,"name":"sally","picture_xl":"http://x/pic.jpg"}}
        ],"total":2}"#,
        NO_HITS,
    );
    let deezer = Deezer::new(&base);
    let found = deezer
        .find(
            "Sally",
            &clues(&["underdogs", "Call Me Nihilist"], &["night"]),
        )
        .unwrap()
        .expect("a match");
    assert_eq!(found.artist_id, 2);
    assert_eq!(found.url, "http://x/pic.jpg");

    let paths = seen.lock().unwrap().clone();
    assert_eq!(
        paths.len(),
        1,
        "stops at the first verified album: {paths:?}"
    );
    assert!(
        paths[0].starts_with("/search/album?q=artist%3A%22Sally%22%20album%3A%22underdogs%22"),
        "{paths:?}"
    );
}

#[test]
fn songs_verify_artists_known_only_from_compilations() {
    // The album is a compilation ("Various Artists"); one of the songs is theirs.
    let (base, seen) = server(
        r#"{"data":[{"title":"Misatocore, Vol. 2","artist":{"id":9,"name":"Various Artists","picture_xl":"http://x/va.jpg"}}]}"#,
        r#"{"data":[{"title":"Loser","artist":{"id":7,"name":"Loser64","picture_xl":"http://x/pic.jpg"}}]}"#,
    );
    let found = Deezer::new(&base)
        .find("Loser64", &clues(&["Misatocore, Vol. 2"], &["Loser"]))
        .unwrap()
        .expect("a match");
    assert_eq!(found.artist_id, 7);
    let paths = seen.lock().unwrap().clone();
    assert!(paths[0].starts_with("/search/album") && paths[1].starts_with("/search/track"));
}

#[test]
fn no_verified_title_means_no_picture() {
    // Deezer knows a Sally, but not with any of our titles: better no picture than a stranger.
    let (base, seen) = server(
        r#"{"data":[{"title":"Greatest Hits","artist":{"id":1,"name":"Sally","picture_xl":"http://x/famous.jpg"}}]}"#,
        r#"{"data":[{"title":"Something Else","artist":{"id":1,"name":"Sally","picture_xl":"http://x/famous.jpg"}}]}"#,
    );
    let deezer = Deezer::new(&base);
    assert_eq!(
        deezer
            .find(
                "Sally",
                &clues(&["a", "b", "c", "d"], &["e", "f", "g", "h"])
            )
            .unwrap(),
        None
    );
    assert_eq!(seen.lock().unwrap().len(), 6, "three of each kind at most");
    assert_eq!(
        deezer.find("Sally", &Clues::default()).unwrap(),
        None,
        "nothing to check by"
    );
}

#[test]
fn the_right_artist_without_a_picture_is_no_picture() {
    let (base, _) = server(
        r#"{"data":[{"title":"underdogs","artist":{"id":2,"name":"sally","picture_xl":"https://e-cdns-images.dzcdn.net/images/artist//1000x1000-000000-80-0-0.jpg"}}]}"#,
        NO_HITS,
    );
    assert_eq!(
        Deezer::new(&base)
            .find("Sally", &clues(&["underdogs"], &[]))
            .unwrap(),
        None
    );
}

#[test]
fn quota_errors_ask_to_slow_down() {
    let (base, _) = server(
        r#"{"error":{"type":"Exception","message":"Quota limit exceeded","code":4}}"#,
        NO_HITS,
    );
    assert!(matches!(
        Deezer::new(&base).find("Anyone", &clues(&["x"], &[])),
        Err(DeezerError::Quota)
    ));
}

#[test]
fn pictures_download_and_missing_ones_fail() {
    let (base, _) = server(NO_HITS, NO_HITS);
    let deezer = Deezer::new(&base);
    assert_eq!(
        &deezer.download(&format!("{base}/pic.jpg")).unwrap()[..3],
        &[0xFF, 0xD8, 0xFF]
    );
    assert!(deezer.download(&format!("{base}/gone.jpg")).is_err());
}
