//! Keeps Windows from sleeping while music plays (the display may still turn off).

use windows::Win32::System::Power::{ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState};

/// Called from the engine thread, which lives as long as the app; the request stays in
/// effect until the same thread clears it.
pub fn keep_awake(playing: bool) {
    let flags = if playing {
        ES_CONTINUOUS | ES_SYSTEM_REQUIRED
    } else {
        ES_CONTINUOUS
    };
    unsafe {
        SetThreadExecutionState(flags);
    }
}
