//! The client against a stand-in Deezer server.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use pp_deezer::{Deezer, DeezerError};

/// Answers `/search/artist` with `search` and `/pic.jpg` with a few bytes; records paths.
fn server(search: &'static str) -> (String, Arc<Mutex<Vec<String>>>) {
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
            let (kind, body): (&str, Vec<u8>) = if path.starts_with("/search/artist") {
                ("application/json", search.as_bytes().to_vec())
            } else if path == "/pic.jpg" {
                ("image/jpeg", vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3])
            } else {
                ("text/plain", b"nope".to_vec())
            };
            let status = if kind == "text/plain" {
                "404 Not Found"
            } else {
                "200 OK"
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

#[test]
fn finds_the_exact_artist_and_downloads_the_picture() {
    let (base, seen) = server(
        r#"{"data":[
            {"id":1,"name":"Sigur Rós Tribute","picture_xl":"http://placeholder/other.jpg","nb_fan":99},
            {"id":2,"name":"Sigur Rós","picture_xl":"http://placeholder/pic.jpg","nb_fan":5}
        ],"total":2}"#,
    );
    let deezer = Deezer::new(&base);
    let found = deezer.find("Sigur Ros").unwrap().expect("a match");
    assert_eq!(found.artist_id, 2);
    assert_eq!(found.url, "http://placeholder/pic.jpg");
    let bytes = deezer.download(&format!("{base}/pic.jpg")).unwrap();
    assert_eq!(&bytes[..3], &[0xFF, 0xD8, 0xFF]);

    let paths = seen.lock().unwrap().clone();
    assert!(
        paths[0].starts_with("/search/artist?q=Sigur%20Ros"),
        "{paths:?}"
    );
}

#[test]
fn no_exact_match_is_no_picture() {
    let (base, _) =
        server(r#"{"data":[{"id":1,"name":"Someone Else","picture_xl":"http://x/a.jpg"}]}"#);
    assert_eq!(Deezer::new(&base).find("Nobody Known").unwrap(), None);
    let (base, _) = server(r#"{"data":[],"total":0}"#);
    assert_eq!(Deezer::new(&base).find("Nobody Known").unwrap(), None);
}

#[test]
fn quota_errors_ask_to_slow_down() {
    let (base, _) =
        server(r#"{"error":{"type":"Exception","message":"Quota limit exceeded","code":4}}"#);
    assert!(matches!(
        Deezer::new(&base).find("Anyone"),
        Err(DeezerError::Quota)
    ));
    let (base, _) =
        server(r#"{"error":{"type":"Exception","message":"Something else","code":800}}"#);
    assert!(matches!(
        Deezer::new(&base).find("Anyone"),
        Err(DeezerError::Api(_))
    ));
}

#[test]
fn a_missing_picture_is_an_error() {
    let (base, _) = server("{}");
    assert!(
        Deezer::new(&base)
            .download(&format!("{base}/gone.jpg"))
            .is_err()
    );
}
