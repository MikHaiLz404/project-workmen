// Workmen desktop binary entry point. The actual Tauri builder
// lives in `lib.rs`; this file just calls into it so the
// `workmen-desktop` binary is the host that Tauri drives.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    workmen_desktop_lib::run();
}
