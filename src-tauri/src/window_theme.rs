//! Native title bar colours (DWM). Caption and text colours need Windows 11; on Windows 10
//! only the dark-mode flag applies and the other calls fail harmlessly.

use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::Graphics::Dwm::{
    DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
};

fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// COLORREF is 0x00BBGGRR.
fn colorref((r, g, b): (u8, u8, u8)) -> COLORREF {
    COLORREF(u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16))
}

pub fn apply(window: &tauri::WebviewWindow, caption: &str, dark: bool) -> Result<(), String> {
    let rgb = parse_hex(caption).ok_or_else(|| format!("not a #rrggbb colour: {caption}"))?;
    // Tauri may link a different `windows` crate version; go through the raw handle.
    let raw = window.hwnd().map_err(|e| e.to_string())?.0;
    let hwnd = HWND(raw);

    let dark_flag: i32 = dark.into();
    let caption_color = colorref(rgb);
    let text_color = if dark {
        colorref((0xE6, 0xE1, 0xE5))
    } else {
        colorref((0x1C, 0x1B, 0x1F))
    };
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&raw const dark_flag).cast(),
            size_of::<i32>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            (&raw const caption_color).cast(),
            size_of::<COLORREF>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TEXT_COLOR,
            (&raw const text_color).cast(),
            size_of::<COLORREF>() as u32,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorref_is_bgr() {
        assert_eq!(colorref(parse_hex("#112233").unwrap()).0, 0x0033_2211);
        assert!(parse_hex("112233").is_none());
    }
}
