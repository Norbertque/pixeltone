//! LRC: `[mm:ss.xx]` timed lines, `[offset:±ms]`, enhanced `<mm:ss.xx>` word stamps
//! (dropped) and metadata tags (ignored). Text without any timestamps is plain lyrics.

use crate::LyricLine;

/// `mm:ss`, `mm:ss.x…`, `mm:ss:xx` (some taggers), minutes may exceed 59.
fn parse_time(tag: &str) -> Option<u32> {
    let (min, rest) = tag.split_once(':')?;
    let min: u32 = min.trim().parse().ok()?;
    let (sec, frac) = match rest.split_once(['.', ':']) {
        Some((s, f)) => (s, f),
        None => (rest, ""),
    };
    let sec: u32 = sec.trim().parse().ok()?;
    if sec >= 60 || !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    // Fraction digits: 1 → tenths, 2 → hundredths, 3 → ms.
    let frac_ms = match frac.len() {
        0 => 0,
        1 => frac.parse::<u32>().ok()? * 100,
        2 => frac.parse::<u32>().ok()? * 10,
        _ => frac[..3].parse::<u32>().ok()?,
    };
    Some((min * 60 + sec) * 1000 + frac_ms)
}

/// Removes enhanced-LRC word stamps like `<01:02.03>`.
fn strip_word_stamps(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        match rest[start..].find('>') {
            Some(len) if parse_time(&rest[start + 1..start + len]).is_some() => {
                out.push_str(&rest[..start]);
                rest = &rest[start + len + 1..];
            }
            _ => {
                out.push_str(&rest[..=start]);
                rest = &rest[start + 1..];
            }
        }
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parsed lines and whether they are synced (timed).
pub fn parse(text: &str) -> (Vec<LyricLine>, bool) {
    let mut timed: Vec<LyricLine> = Vec::new();
    let mut plain: Vec<LyricLine> = Vec::new();
    let mut offset: i64 = 0;
    for raw in text.lines() {
        let mut line = raw.trim();
        let mut times = Vec::new();
        let mut tag_only = false;
        while let Some(rest) = line.strip_prefix('[') {
            let Some(end) = rest.find(']') else { break };
            let tag = &rest[..end];
            if let Some(t) = parse_time(tag) {
                times.push(t);
            } else if let Some((key, value)) = tag.split_once(':') {
                if key.trim().eq_ignore_ascii_case("offset") {
                    offset = value.trim().parse().unwrap_or(0);
                }
                tag_only = true;
            } else {
                break;
            }
            line = rest[end + 1..].trim_start();
        }
        if !times.is_empty() {
            let text = strip_word_stamps(line);
            for t in times {
                timed.push(LyricLine {
                    time_ms: Some(t),
                    text: text.clone(),
                });
            }
        } else if !tag_only {
            plain.push(LyricLine {
                time_ms: None,
                text: raw.trim().to_string(),
            });
        }
    }
    if timed.is_empty() {
        // Trim blank lines at the ends of plain lyrics.
        while plain.last().is_some_and(|l| l.text.is_empty()) {
            plain.pop();
        }
        let first = plain
            .iter()
            .position(|l| !l.text.is_empty())
            .unwrap_or(plain.len());
        return (plain.split_off(first), false);
    }
    // A positive offset makes lyrics appear earlier.
    for line in &mut timed {
        line.time_ms = line
            .time_ms
            .map(|t| (i64::from(t) - offset).clamp(0, i64::from(u32::MAX)) as u32);
    }
    timed.sort_by_key(|l| l.time_ms);
    (timed, true)
}

/// `[mm:ss.xx]`.
pub fn stamp(ms: u32) -> String {
    let cs = ms / 10;
    format!("[{:02}:{:02}.{:02}]", cs / 6000, (cs / 100) % 60, cs % 100)
}

/// Lines back to LRC text (plain lines stay plain).
pub fn render(lines: &[LyricLine]) -> String {
    lines
        .iter()
        .map(|l| match l.time_ms {
            Some(t) => format!("{}{}", stamp(t), l.text),
            None => l.text.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn times(lines: &[LyricLine]) -> Vec<(Option<u32>, &str)> {
        lines.iter().map(|l| (l.time_ms, l.text.as_str())).collect()
    }

    #[test]
    fn parses_timed_lines_with_repeats_and_metadata() {
        let text = "[ti:Song]\n[ar:Someone]\n[00:12.50]First\n[00:05.1][01:00.123]Chorus\n\n[02:00]  Last  \nnoise";
        let (lines, synced) = parse(text);
        assert!(synced);
        assert_eq!(
            times(&lines),
            [
                (Some(5_100), "Chorus"),
                (Some(12_500), "First"),
                (Some(60_123), "Chorus"),
                (Some(120_000), "Last"),
            ]
        );
    }

    #[test]
    fn applies_offset_and_strips_word_stamps() {
        let (lines, _) =
            parse("[offset:+500]\n[00:01.00]<00:01.00>Hel <00:01.50>lo\n[00:00.20]start");
        assert_eq!(times(&lines), [(Some(0), "start"), (Some(500), "Hel lo")]);
    }

    #[test]
    fn plain_text_stays_plain() {
        let (lines, synced) = parse("\n\nVerse one\nline two\n\nChorus\n\n");
        assert!(!synced);
        assert_eq!(
            lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
            ["Verse one", "line two", "", "Chorus"]
        );
    }

    #[test]
    fn rejects_bad_stamps_and_round_trips() {
        assert_eq!(parse_time("01:75.00"), None);
        assert_eq!(parse_time("ab:10"), None);
        assert_eq!(parse_time("90:01:5"), Some(5_401_500));
        assert_eq!(stamp(83_456), "[01:23.45]");
        let (lines, _) = parse("[01:23.45]A\n[01:24.00]B");
        assert_eq!(render(&lines), "[01:23.45]A\n[01:24.00]B");
    }
}
