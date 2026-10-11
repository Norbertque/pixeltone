//! Watches the library folders and asks for a rescan when music or covers change.

use std::path::PathBuf;
use std::time::Duration;

use notify_debouncer_full::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

use crate::scan::is_audio;

/// Changes are coalesced until things have been quiet this long.
const QUIET: Duration = Duration::from_secs(2);

pub struct FolderWatcher {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

fn relevant(path: &std::path::Path) -> bool {
    // Folders (renames, deletions) and audio files; image changes may be new covers.
    is_audio(path)
        || path.extension().is_none()
        || path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .is_some_and(|e| matches!(e.as_str(), "jpg" | "jpeg" | "png" | "webp"))
}

impl FolderWatcher {
    pub fn new(roots: &[PathBuf], on_change: impl Fn() + Send + 'static) -> Result<Self, String> {
        let mut debouncer = new_debouncer(QUIET, None, move |result: DebounceEventResult| {
            if let Ok(events) = result
                && events.iter().any(|e| e.paths.iter().any(|p| relevant(p)))
            {
                on_change();
            }
        })
        .map_err(|e| e.to_string())?;
        for root in roots.iter().filter(|r| r.is_dir()) {
            debouncer
                .watch(root, RecursiveMode::Recursive)
                .map_err(|e| format!("{}: {e}", root.display()))?;
        }
        Ok(Self {
            _debouncer: debouncer,
        })
    }
}
