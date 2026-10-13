//! The notification-area icon (playback controls, the current song as its tooltip) and the
//! mini player: a small always-on-top window that stands in for the main one.

use pp_engine::{PlaybackStatus, PlayerSnapshot};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

use crate::AppState;

pub const MINI: &str = "mini";
const MINI_SIZE: (f64, f64) = (420.0, 128.0);

pub struct Tray {
    icon: TrayIcon<Wry>,
    toggle: MenuItem<Wry>,
}

impl Tray {
    pub fn create(app: &AppHandle) -> tauri::Result<Self> {
        let toggle = MenuItem::with_id(app, "toggle", "Play", true, None::<&str>)?;
        let menu = Menu::with_items(
            app,
            &[
                &toggle,
                &MenuItem::with_id(app, "next", "Next", true, None::<&str>)?,
                &MenuItem::with_id(app, "previous", "Previous", true, None::<&str>)?,
                &PredefinedMenuItem::separator(app)?,
                &MenuItem::with_id(app, "mini", "Mini player", true, None::<&str>)?,
                &MenuItem::with_id(app, "show", "Show PixelPlayer", true, None::<&str>)?,
                &PredefinedMenuItem::separator(app)?,
                &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
            ],
        )?;
        let mut builder = TrayIconBuilder::with_id("main")
            .tooltip("PixelPlayer PC")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(
                |app, event| match (event.id.as_ref(), app.try_state::<AppState>()) {
                    ("toggle", Some(s)) => s.engine.toggle(),
                    ("next", Some(s)) => s.engine.next(),
                    ("previous", Some(s)) => s.engine.previous(),
                    ("mini", _) => open_mini(app),
                    ("show", _) => show_main(app),
                    ("quit", _) => app.exit(0),
                    _ => {}
                },
            )
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    show_main(tray.app_handle());
                }
            });
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone());
        }
        let icon = builder.build(app)?;
        Ok(Self { icon, toggle })
    }

    /// Follows the player: the song as the tooltip, Play or Pause in the menu.
    pub fn update(&self, snapshot: &PlayerSnapshot) {
        let tooltip = match &snapshot.track {
            Some(t) => match t.artist.as_deref().filter(|a| !a.is_empty()) {
                Some(artist) => format!("{} – {artist}", t.title),
                None => t.title.clone(),
            },
            None => "PixelPlayer PC".to_string(),
        };
        // Windows cuts tooltips off at 127 characters.
        let tooltip: String = tooltip.chars().take(120).collect();
        let _ = self.icon.set_tooltip(Some(tooltip));
        let playing = snapshot.status == PlaybackStatus::Playing;
        let _ = self.toggle.set_text(if playing { "Pause" } else { "Play" });
    }
}

/// Brings the main window back (from the tray or the mini player), closing the mini one.
pub fn show_main(app: &AppHandle) {
    if let Some(mini) = app.get_webview_window(MINI) {
        let _ = mini.close();
    }
    reveal_main(app);
}

/// Shows the main window, wherever it was (hidden, minimised, behind others).
pub fn reveal_main(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
}

/// Swaps the main window for the mini player.
pub fn open_mini(app: &AppHandle) {
    let window = match app.get_webview_window(MINI) {
        Some(w) => Ok(w),
        None => WebviewWindowBuilder::new(app, MINI, WebviewUrl::App("index.html#mini".into()))
            .title("PixelPlayer mini player")
            .inner_size(MINI_SIZE.0, MINI_SIZE.1)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .decorations(false)
            .shadow(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .build(),
    };
    match window {
        Ok(w) => {
            let _ = w.show();
            let _ = w.set_focus();
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.hide();
            }
        }
        Err(e) => tracing::warn!("can't open the mini player: {e}"),
    }
}

#[tauri::command]
pub fn open_mini_player(app: AppHandle) {
    open_mini(&app);
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) {
    show_main(&app);
}
