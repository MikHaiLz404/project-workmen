//! Workmen desktop library. Hosts the Tauri 2 builder and the
//! typed command surface for the React shell.
//!
//! The T2.T1 plan calls for two Tauri commands:
//! - `get_system_info` -- reports host OS, arch, and Workmen
//!   version. The shell uses it to display the backend-status
//!   badge.
//! - `get_app_log_directory` -- returns the OS app-data log
//!   directory. The shell uses it to open the log viewer.
//!
//! Both are read-only.

pub mod commands;

use tauri::Manager;

/// Run the Tauri application. Called from `main.rs`.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::system::get_system_info,
            commands::system::get_app_log_directory,
        ])
        .setup(|app| {
            // Log the app-data directory at startup so the bottom
            // console has something to show even before the user
            // opens a project. The Tauri Manager resolves the
            // path lazily -- a missing app-data directory is not
            // an error; the Manager will create it on first write.
            if let Ok(dir) = app.path().app_log_dir() {
                eprintln!("[workmen] app log dir: {}", dir.display());
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running workmen-desktop");
}
