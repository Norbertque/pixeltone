//! Incremental scanning: walk the library folders, compare with the database, read only new
//! or changed files (in parallel), write them in batches, and drop files that disappeared.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::artwork::{ArtStore, find_sidecar};
use crate::db::{Db, FileStat, ScannedFile};
use crate::model::{ScanPhase, ScanProgress};
use crate::tags::read_track;

/// Extensions the scanner picks up (anything FFmpeg decodes would play, but these are music).
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "flac", "m4a", "mp4", "m4b", "alac", "wav", "wave", "aif", "aiff", "aifc", "mp3", "mp2", "aac",
    "ogg", "oga", "opus", "ape", "wv", "wma", "dsf", "dff", "mka", "tak", "mpc",
];

/// Files read and written per transaction.
const BATCH: usize = 128;

pub fn is_audio(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.as_str()))
}

pub fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.depth() > 0 && entry.file_name().to_string_lossy().starts_with('.')
}

/// Audio files below `root`, or `None` if the root itself is unreachable (an offline drive
/// must not look like a library whose files were all deleted).
fn walk(root: &Path, cancel: &AtomicBool) -> Option<Vec<FileStat>> {
    if !root.is_dir() {
        return None;
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .flatten()
    {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if !entry.file_type().is_file() || !is_audio(entry.path()) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let path = entry.path();
        files.push(FileStat {
            path: path.to_string_lossy().into_owned(),
            dir: path
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            mtime,
            size: meta.len() as i64,
        });
    }
    Some(files)
}

/// Cover of a folder's image file, read once per folder per scan.
fn sidecar_cover(
    cache: &Mutex<HashMap<String, Option<String>>>,
    dir: &str,
    art: &ArtStore,
) -> Option<String> {
    if let Some(hit) = cache.lock().get(dir) {
        return hit.clone();
    }
    let hash = find_sidecar(Path::new(dir))
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|bytes| art.store(&bytes).ok());
    cache.lock().insert(dir.to_string(), hash.clone());
    hash
}

pub fn scan(
    db: &mut Db,
    art: &ArtStore,
    delimiters: &[String],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(&ScanProgress),
) -> rusqlite::Result<ScanProgress> {
    let mut p = ScanProgress {
        phase: ScanPhase::Walking,
        done: 0,
        total: 0,
        added: 0,
        updated: 0,
        removed: 0,
        errors: 0,
    };
    progress(&p);

    let mut work: Vec<(u32, Vec<FileStat>)> = Vec::new();
    let mut stale: Vec<u32> = Vec::new();
    for folder in db.folders()? {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let Some(files) = walk(Path::new(&folder.path), cancel) else {
            tracing::warn!(
                "library folder unreachable, keeping its tracks: {}",
                folder.path
            );
            continue;
        };
        let known = db.known_files(folder.id)?;
        let seen: HashSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
        stale.extend(
            known
                .iter()
                .filter(|(path, _)| !seen.contains(path.as_str()))
                .map(|(_, (id, _, _))| *id),
        );
        let changed: Vec<FileStat> = files
            .into_iter()
            .filter(|f| {
                known
                    .get(&f.path)
                    .is_none_or(|&(_, mtime, size)| mtime != f.mtime || size != f.size)
            })
            .collect();
        work.push((folder.id, changed));
    }

    p.phase = ScanPhase::Reading;
    p.total = work.iter().map(|(_, f)| f.len() as u32).sum();
    progress(&p);

    let sidecars = Mutex::new(HashMap::new());
    let now = now_secs();
    'folders: for (folder_id, files) in work {
        for chunk in files.chunks(BATCH) {
            if cancel.load(Ordering::Relaxed) {
                break 'folders;
            }
            let read: Vec<Option<ScannedFile>> = chunk
                .par_iter()
                .map(|stat| match read_track(Path::new(&stat.path), delimiters) {
                    Ok(mut track) => {
                        let cover = match track.cover.take() {
                            Some(bytes) => art.store(&bytes).ok(),
                            None => sidecar_cover(&sidecars, &stat.dir, art),
                        };
                        Some(ScannedFile {
                            stat: stat.clone(),
                            track,
                            cover,
                        })
                    }
                    Err(e) => {
                        tracing::warn!("can't read {}: {e}", stat.path);
                        None
                    }
                })
                .collect();
            let errors = read.iter().filter(|r| r.is_none()).count() as u32;
            let files: Vec<ScannedFile> = read.into_iter().flatten().collect();
            let (added, updated) = db.write_tracks(folder_id, &files, now)?;
            p.done += chunk.len() as u32;
            p.added += added;
            p.updated += updated;
            p.errors += errors;
            progress(&p);
        }
    }

    if !stale.is_empty() && !cancel.load(Ordering::Relaxed) {
        db.remove_tracks(&stale)?;
        p.removed = stale.len() as u32;
    }
    p.phase = ScanPhase::Finishing;
    progress(&p);
    db.finish_scan()?;
    p.phase = ScanPhase::Done;
    progress(&p);
    Ok(p)
}
