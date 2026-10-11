//! Artist splitting ("smart artist parsing"): multi-value tags plus configurable delimiters.

/// Splits on these by default (matched ignoring ASCII case). `&` and `,` are left out on
/// purpose so "Simon & Garfunkel" or "Crosby, Stills, Nash & Young" survive; users can add them.
pub const DEFAULT_DELIMITERS: &[&str] = &[
    ";",
    "/",
    " feat. ",
    " ft. ",
    " featuring ",
    " feat ",
    " ft ",
    "(feat. ",
    "(ft. ",
    "[feat. ",
    "[ft. ",
];

/// Names that contain a delimiter but are one artist.
pub const PROTECTED_NAMES: &[&str] = &["AC/DC"];

pub fn default_delimiters() -> Vec<String> {
    DEFAULT_DELIMITERS.iter().map(|d| d.to_string()).collect()
}

/// Lower-cased, whitespace-collapsed form used to match names.
pub fn norm(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Splits `s` on `delimiter`, ignoring ASCII case (delimiters are ASCII).
fn split_ignore_case(s: &str, delimiter: &str) -> Vec<String> {
    if delimiter.is_empty() {
        return vec![s.to_string()];
    }
    let haystack = s.to_ascii_lowercase();
    let needle = delimiter.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut start = 0;
    while let Some(i) = haystack[start..].find(&needle) {
        out.push(s[start..start + i].to_string());
        start += i + needle.len();
    }
    out.push(s[start..].to_string());
    out
}

/// Replaces protected names with placeholders that no delimiter can match.
fn protect(s: &str) -> String {
    let mut out = s.to_string();
    for (i, name) in PROTECTED_NAMES.iter().enumerate() {
        let parts = split_ignore_case(&out, name);
        out = parts.join(&format!("\u{1}{i}\u{1}"));
    }
    out
}

fn restore(s: &str) -> String {
    let mut out = s.to_string();
    for (i, name) in PROTECTED_NAMES.iter().enumerate() {
        out = out.replace(&format!("\u{1}{i}\u{1}"), name);
    }
    out
}

/// Splits artist tag values into individual names, de-duplicated in order.
pub fn split_artists<S: AsRef<str>>(values: &[S], delimiters: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for value in values {
        let mut parts = vec![protect(value.as_ref())];
        for d in delimiters {
            parts = parts.iter().flat_map(|p| split_ignore_case(p, d)).collect();
        }
        for part in parts {
            let name =
                restore(part.trim_matches(|c: char| {
                    c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']')
                }));
            if !name.is_empty() && !out.iter().any(|o| norm(o) == norm(&name)) {
                out.push(name);
            }
        }
    }
    out
}

/// Genres split on the usual separators.
pub fn split_genres<S: AsRef<str>>(values: &[S]) -> Vec<String> {
    let separators = [";", "/", ",", "\\"].map(String::from);
    split_artists(values, &separators)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_default_delimiters() {
        let d = default_delimiters();
        assert_eq!(split_artists(&["A; B / C"], &d), ["A", "B", "C"]);
        assert_eq!(
            split_artists(&["Daft Punk feat. Pharrell Williams"], &d),
            ["Daft Punk", "Pharrell Williams"]
        );
        assert_eq!(
            split_artists(&["Artist A (Feat. Artist B)"], &d),
            ["Artist A", "Artist B"]
        );
        assert_eq!(split_artists(&["DJ X Ft Y"], &d), ["DJ X", "Y"]);
    }

    #[test]
    fn keeps_band_names_together() {
        let d = default_delimiters();
        assert_eq!(
            split_artists(&["Simon & Garfunkel"], &d),
            ["Simon & Garfunkel"]
        );
        assert_eq!(
            split_artists(&["Crosby, Stills, Nash & Young"], &d),
            ["Crosby, Stills, Nash & Young"]
        );
        assert_eq!(split_artists(&["AC/DC"], &d), ["AC/DC"]);
        assert_eq!(split_artists(&["ac/dc; Foo"], &d), ["AC/DC", "Foo"]);
    }

    #[test]
    fn merges_multi_value_tags_without_duplicates() {
        let d = default_delimiters();
        assert_eq!(
            split_artists(&["Björk", "björk", "Thom Yorke"], &d),
            ["Björk", "Thom Yorke"]
        );
    }

    #[test]
    fn user_delimiters_apply() {
        let d = vec!["&".to_string()];
        assert_eq!(
            split_artists(&["Simon & Garfunkel"], &d),
            ["Simon", "Garfunkel"]
        );
    }

    #[test]
    fn genres() {
        assert_eq!(
            split_genres(&["Rock; Pop/Indie, Folk"]),
            ["Rock", "Pop", "Indie", "Folk"]
        );
    }
}
