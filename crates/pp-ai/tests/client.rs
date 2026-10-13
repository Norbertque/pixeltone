//! The playlist client against a stand-in Messages API.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use pp_ai::{AiError, Catalog, Claude, Song};

struct Seen {
    headers: Vec<String>,
    body: String,
}

/// Answers every request with `status` and `body`; records what was sent.
fn server(status: &'static str, body: String) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut headers = Vec::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let line = line.trim_end().to_string();
                if line.is_empty() {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = v.trim().parse().unwrap();
                }
                headers.push(line);
            }
            let mut sent = vec![0; length];
            reader.read_exact(&mut sent).unwrap();
            log.lock().unwrap().push(Seen {
                headers,
                body: String::from_utf8(sent).unwrap(),
            });
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (base, seen)
}

fn answer(text: &str, stop: &str) -> String {
    serde_json::json!({
        "id": "msg_1",
        "type": "message",
        "role": "assistant",
        "model": "claude-opus-5-5",
        "content": [
            {"type": "thinking", "thinking": "", "signature": "x"},
            {"type": "text", "text": text}
        ],
        "stop_reason": stop,
        "usage": {"input_tokens": 10, "output_tokens": 20}
    })
    .to_string()
}

fn library() -> Catalog {
    let songs: Vec<Song> = (1..=4)
        .map(|id| Song {
            id,
            title: format!("Song {id}"),
            artist: "Artist".into(),
            album: "Album".into(),
            year: None,
            genres: String::new(),
        })
        .collect();
    Catalog::new(&songs)
}

#[test]
fn a_playlist_comes_back_with_only_songs_from_the_library() {
    let reply =
        r#"{"name":"\"Night Drive\"","description":"Late and calm.","track_ids":[3,99,1,3,2]}"#;
    let (base, seen) = server("200 OK", answer(reply, "end_turn"));
    let playlist = Claude::new(&base, " sk-test ")
        .playlist(&library(), "calm night drive", 25)
        .unwrap();
    assert_eq!(playlist.name, "Night Drive");
    assert_eq!(
        playlist.track_ids,
        [3, 1, 2],
        "unknown and repeated ids dropped"
    );

    let seen = seen.lock().unwrap();
    let headers = seen[0].headers.join("\n").to_ascii_lowercase();
    assert!(
        headers.contains("x-api-key: sk-test"),
        "key trimmed and sent"
    );
    assert!(headers.contains("anthropic-version: 2023-06-01"));
    assert!(headers.contains("anthropic-beta: server-side-fallback-2026-07-01"));
    let body: serde_json::Value = serde_json::from_str(&seen[0].body).unwrap();
    assert_eq!(
        body["messages"][0]["content"],
        "Request: calm night drive\nNumber of songs: 25"
    );
}

#[test]
fn asking_for_fewer_songs_caps_the_answer() {
    let reply = r#"{"name":"x","description":"y","track_ids":[1,2,3,4]}"#;
    let (base, _) = server("200 OK", answer(reply, "end_turn"));
    let playlist = Claude::new(&base, "k")
        .playlist(&library(), "p", 2)
        .unwrap();
    assert_eq!(playlist.track_ids, [1, 2]);
}

#[test]
fn errors_say_what_to_do() {
    let error = |kind: &str, message: &str| {
        serde_json::json!({"type": "error", "error": {"type": kind, "message": message}})
            .to_string()
    };
    let cases = [
        (
            "401 Unauthorized",
            error("authentication_error", "invalid x-api-key"),
            "BadKey",
        ),
        (
            "429 Too Many Requests",
            error("rate_limit_error", "slow down"),
            "Busy",
        ),
        ("529 Overloaded", error("overloaded_error", "busy"), "Busy"),
        (
            "400 Bad Request",
            error("invalid_request_error", "bad thing"),
            "Api",
        ),
    ];
    for (status, body, want) in cases {
        let (base, _) = server(status, body);
        let err = Claude::new(&base, "k")
            .playlist(&library(), "p", 5)
            .unwrap_err();
        assert_eq!(
            format!("{err:?}").split('(').next().unwrap(),
            want,
            "{status}"
        );
    }

    let (base, _) = server("200 OK", answer("", "refusal"));
    assert!(matches!(
        Claude::new(&base, "k").playlist(&library(), "p", 5),
        Err(AiError::Refused)
    ));
    let (base, _) = server("200 OK", answer(r#"{"name":"x","descr"#, "max_tokens"));
    assert!(matches!(
        Claude::new(&base, "k").playlist(&library(), "p", 5),
        Err(AiError::Truncated)
    ));
}
