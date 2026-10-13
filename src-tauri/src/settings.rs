//! Persistent app settings (`settings.json` in the app config directory).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pp_core::gain;
use pp_engine::OutputSettings;
use serde::{Deserialize, Serialize};

/// ReplayGain pre-amp range, dB either way.
pub const MAX_PREAMP_DB: f32 = 15.0;
/// Longest crossfade, ms.
pub const MAX_CROSSFADE_MS: u32 = 12_000;

fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// Per-device volume policy for exclusive mode without hardware volume.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DevicePrefs {
    pub software_gain_db: f32,
    /// Full-scale output, for DACs feeding an external amplifier.
    pub line_level: bool,
    /// Attenuate in the app although the device has its own volume control.
    pub software_volume: bool,
    /// The user accepted the loudness warning, unlocking levels above −10 dB.
    pub loudness_acknowledged: bool,
}

impl Default for DevicePrefs {
    fn default() -> Self {
        Self {
            software_gain_db: gain::DEFAULT_DB,
            line_level: false,
            software_volume: false,
            loudness_acknowledged: false,
        }
    }
}

/// What the app may do online, and other app-level choices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    /// Look lyrics up on LRCLIB when a track has none of its own.
    pub lyrics_online: bool,
    /// Download artist pictures from Deezer.
    pub artist_images_online: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            lyrics_online: true,
            artist_images_online: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub output: OutputSettings,
    /// Keyed by device id; the empty key is "the Windows default device".
    pub devices: BTreeMap<String, DevicePrefs>,
    pub preferences: Preferences,
}

impl AppSettings {
    pub fn device_key(device_id: Option<&str>) -> String {
        device_id.unwrap_or_default().to_string()
    }

    pub fn prefs(&self, device_id: Option<&str>) -> DevicePrefs {
        self.devices
            .get(&Self::device_key(device_id))
            .cloned()
            .unwrap_or_default()
    }

    /// Output settings with the selected device's volume policy applied and clamped.
    pub fn effective_output(&self) -> OutputSettings {
        let prefs = self.prefs(self.output.device_id.as_deref());
        OutputSettings {
            software_gain_db: gain::clamp_to_policy(
                prefs.software_gain_db,
                prefs.loudness_acknowledged,
            ),
            line_level: prefs.line_level,
            software_volume: prefs.software_volume,
            shared_gain_db: finite(self.output.shared_gain_db).clamp(gain::FLOOR_DB, 0.0),
            buffer_ms: self.output.buffer_ms.clamp(10, 500),
            eq: self.output.eq.clone().normalized(),
            replay_gain_preamp_db: finite(self.output.replay_gain_preamp_db)
                .clamp(-MAX_PREAMP_DB, MAX_PREAMP_DB),
            crossfade_ms: self.output.crossfade_ms.min(MAX_CROSSFADE_MS),
            ..self.output.clone()
        }
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// Writes atomically (temp file + rename) so a crash can't leave half a file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp: PathBuf = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?,
        )?;
        std::fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_is_capped_until_acknowledged() {
        let mut s = AppSettings::default();
        s.devices.insert(
            String::new(),
            DevicePrefs {
                software_gain_db: -3.0,
                ..DevicePrefs::default()
            },
        );
        assert_eq!(s.effective_output().software_gain_db, gain::LOCKED_MAX_DB);
        s.devices.get_mut("").unwrap().loudness_acknowledged = true;
        assert_eq!(s.effective_output().software_gain_db, -3.0);
    }

    #[test]
    fn hand_edited_values_are_brought_back_in_range() {
        let mut s = AppSettings::default();
        s.output.eq.gains_db = vec![99.0];
        s.output.replay_gain_preamp_db = f32::INFINITY;
        s.output.crossfade_ms = 60_000;
        let out = s.effective_output();
        assert_eq!(out.eq.gains_db.len(), 10);
        assert_eq!(out.eq.gains_db[0], 12.0);
        assert_eq!(out.replay_gain_preamp_db, 0.0);
        assert_eq!(out.crossfade_ms, MAX_CROSSFADE_MS);
    }

    #[test]
    fn round_trips_through_json() {
        let dir = tempfile_dir();
        let path = dir.join("settings.json");
        let mut s = AppSettings::default();
        s.output.exclusive = true;
        s.save(&path).unwrap();
        assert_eq!(AppSettings::load(&path), s);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn tempfile_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pp-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
