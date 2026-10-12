//! System media transport controls: the hardware media keys, and the media flyout Windows
//! shows next to the volume indicator (title, artist, cover, play/pause, next/previous and a
//! seek bar). Compiles to nothing elsewhere.
#![cfg(windows)]

use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows::Media::{
    AutoRepeatModeChangeRequestedEventArgs, MediaPlaybackAutoRepeatMode, MediaPlaybackStatus,
    MediaPlaybackType, PlaybackPositionChangeRequestedEventArgs,
    ShuffleEnabledChangeRequestedEventArgs, SystemMediaTransportControls,
    SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
    SystemMediaTransportControlsTimelineProperties,
};
use windows::Storage::Streams::{
    DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::WinRT::ISystemMediaTransportControlsInterop;
use windows::core::{HSTRING, Ref, Result, factory};

/// What the user asked for through a media key or the flyout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MediaAction {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    /// Seek to this position (ms).
    Seek(f64),
    Shuffle(bool),
    Repeat(Repeat),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repeat {
    Off,
    All,
    One,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Playing,
    Paused,
    Stopped,
}

/// The track the flyout shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Track {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// Encoded cover image (JPEG, PNG…).
    pub cover: Option<Vec<u8>>,
}

pub struct MediaControls {
    smtc: SystemMediaTransportControls,
    shown: Mutex<Option<Track>>,
}

/// Joins the multithreaded COM apartment; call once on a thread that updates the controls.
pub fn init_thread() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
}

fn timespan(ms: f64) -> TimeSpan {
    TimeSpan {
        Duration: (ms.max(0.0) * 10_000.0) as i64,
    }
}

impl MediaControls {
    /// Attaches the controls to a window (`hwnd`: its raw handle). `on_action` runs on a
    /// system thread.
    pub fn new(
        hwnd: isize,
        on_action: impl Fn(MediaAction) + Send + Sync + 'static,
    ) -> Result<Self> {
        let interop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()?;
        let smtc: SystemMediaTransportControls =
            unsafe { interop.GetForWindow(HWND(hwnd as *mut c_void)) }?;
        smtc.SetIsEnabled(false)?;
        smtc.SetIsPlayEnabled(true)?;
        smtc.SetIsPauseEnabled(true)?;
        smtc.SetIsStopEnabled(true)?;

        let on_action = Arc::new(on_action);
        let f = on_action.clone();
        smtc.ButtonPressed(&TypedEventHandler::new(
            move |_, args: Ref<SystemMediaTransportControlsButtonPressedEventArgs>| {
                let action = match args.ok()?.Button()? {
                    SystemMediaTransportControlsButton::Play => MediaAction::Play,
                    SystemMediaTransportControlsButton::Pause => MediaAction::Pause,
                    SystemMediaTransportControlsButton::Stop => MediaAction::Stop,
                    SystemMediaTransportControlsButton::Next => MediaAction::Next,
                    SystemMediaTransportControlsButton::Previous => MediaAction::Previous,
                    _ => return Ok(()),
                };
                f(action);
                Ok(())
            },
        ))?;
        let f = on_action.clone();
        smtc.PlaybackPositionChangeRequested(&TypedEventHandler::new(
            move |_, args: Ref<PlaybackPositionChangeRequestedEventArgs>| {
                let position = args.ok()?.RequestedPlaybackPosition()?;
                f(MediaAction::Seek(position.Duration as f64 / 10_000.0));
                Ok(())
            },
        ))?;
        let f = on_action.clone();
        smtc.ShuffleEnabledChangeRequested(&TypedEventHandler::new(
            move |_, args: Ref<ShuffleEnabledChangeRequestedEventArgs>| {
                f(MediaAction::Shuffle(args.ok()?.RequestedShuffleEnabled()?));
                Ok(())
            },
        ))?;
        let f = on_action;
        smtc.AutoRepeatModeChangeRequested(&TypedEventHandler::new(
            move |_, args: Ref<AutoRepeatModeChangeRequestedEventArgs>| {
                let repeat = match args.ok()?.RequestedAutoRepeatMode()? {
                    MediaPlaybackAutoRepeatMode::Track => Repeat::One,
                    MediaPlaybackAutoRepeatMode::List => Repeat::All,
                    _ => Repeat::Off,
                };
                f(MediaAction::Repeat(repeat));
                Ok(())
            },
        ))?;
        Ok(Self {
            smtc,
            shown: Mutex::new(None),
        })
    }

    /// Shows a track in the flyout (`None` hides the controls).
    pub fn set_track(&self, track: Option<&Track>) -> Result<()> {
        let mut shown = self.shown.lock().unwrap_or_else(|e| e.into_inner());
        if shown.as_ref() == track {
            return Ok(());
        }
        let Some(track) = track else {
            self.smtc.SetIsEnabled(false)?;
            self.smtc.DisplayUpdater()?.ClearAll()?;
            *shown = None;
            return Ok(());
        };
        let updater = self.smtc.DisplayUpdater()?;
        updater.SetType(MediaPlaybackType::Music)?;
        let music = updater.MusicProperties()?;
        music.SetTitle(&HSTRING::from(&track.title))?;
        music.SetArtist(&HSTRING::from(track.artist.as_deref().unwrap_or_default()))?;
        music.SetAlbumTitle(&HSTRING::from(track.album.as_deref().unwrap_or_default()))?;
        match &track.cover {
            Some(bytes) => match thumbnail(bytes) {
                Ok(t) => updater.SetThumbnail(&t)?,
                Err(e) => {
                    tracing::debug!("cover not shown in the media flyout: {e}");
                    updater.SetThumbnail(None::<&RandomAccessStreamReference>)?;
                }
            },
            None => updater.SetThumbnail(None::<&RandomAccessStreamReference>)?,
        }
        updater.Update()?;
        self.smtc.SetIsEnabled(true)?;
        *shown = Some(track.clone());
        Ok(())
    }

    pub fn set_status(&self, status: Status, can_previous: bool, can_next: bool) -> Result<()> {
        self.smtc.SetPlaybackStatus(match status {
            Status::Playing => MediaPlaybackStatus::Playing,
            Status::Paused => MediaPlaybackStatus::Paused,
            Status::Stopped => MediaPlaybackStatus::Stopped,
        })?;
        self.smtc.SetIsPreviousEnabled(can_previous)?;
        self.smtc.SetIsNextEnabled(can_next)
    }

    /// The flyout's seek bar.
    pub fn set_timeline(&self, position_ms: f64, duration_ms: f64) -> Result<()> {
        let timeline = SystemMediaTransportControlsTimelineProperties::new()?;
        timeline.SetStartTime(timespan(0.0))?;
        timeline.SetEndTime(timespan(duration_ms))?;
        timeline.SetMinSeekTime(timespan(0.0))?;
        timeline.SetMaxSeekTime(timespan(duration_ms))?;
        timeline.SetPosition(timespan(position_ms.min(duration_ms)))?;
        self.smtc.UpdateTimelineProperties(&timeline)
    }

    pub fn set_modes(&self, shuffle: bool, repeat: Repeat) -> Result<()> {
        self.smtc.SetShuffleEnabled(shuffle)?;
        self.smtc.SetAutoRepeatMode(match repeat {
            Repeat::Off => MediaPlaybackAutoRepeatMode::None,
            Repeat::All => MediaPlaybackAutoRepeatMode::List,
            Repeat::One => MediaPlaybackAutoRepeatMode::Track,
        })
    }
}

/// An in-memory stream holding the cover image.
fn thumbnail(bytes: &[u8]) -> Result<RandomAccessStreamReference> {
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(bytes)?;
    writer.StoreAsync()?.join()?;
    writer.FlushAsync()?.join()?;
    writer.DetachStream()?;
    stream.Seek(0)?;
    RandomAccessStreamReference::CreateFromStream(&stream)
}
