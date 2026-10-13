//! The tag editor's commands: read the editable fields of some files, write a change into
//! all of them, then let the library and the player catch up.

use std::path::Path;

use pp_library::edit::{TagFields, read_fields, validate, write_fields};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagWriteReport {
    pub written: u32,
    /// "file name: reason" for each file that couldn't be written.
    pub failed: Vec<String>,
}

fn file_name(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}

/// The current fields of each file, in order.
#[tauri::command]
pub async fn tags_read(paths: Vec<String>) -> Result<Vec<TagFields>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .iter()
            .map(|p| read_fields(Path::new(p)).map_err(|e| format!("{}: {e}", file_name(p))))
            .collect()
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tags_write(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
    change: TagFields,
) -> Result<TagWriteReport, String> {
    validate(&change)?;
    let playing = state.engine.snapshot().track.map(|t| t.path);
    let edited = paths.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let mut report = TagWriteReport {
            written: 0,
            failed: Vec::new(),
        };
        for p in &edited {
            match write_fields(Path::new(p), &change) {
                Ok(()) => report.written += 1,
                Err(e) => report.failed.push(format!("{}: {e}", file_name(p))),
            }
        }
        report
    })
    .await
    .map_err(|e| e.to_string())?;
    if report.written > 0 {
        // The playing file may have been rewritten under the decoder: reopen it in place,
        // which also shows the new tags.
        if playing.is_some_and(|p| paths.contains(&p)) {
            state.engine.reload();
        }
        crate::library::scan(&app, &state.library);
    }
    Ok(report)
}
