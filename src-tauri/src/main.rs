// Release builds are GUI-only: no console window behind the app on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    pixelplayer_pc_lib::run();
}
