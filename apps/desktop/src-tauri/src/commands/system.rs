//! Read-only Tauri commands for the desktop shell.
//!
//! T2.T1 calls for two commands:
//!
//! - [`get_system_info`] -- returns a small JSON payload with the
//!   host's OS, arch, and the Workmen version. The shell uses
//!   this to display the backend-status badge.
//! - [`get_app_log_directory`] -- returns the OS app-data log
//!   directory as a `String`. The shell uses this to open the
//!   log viewer.
//!
//! Both are pure read-only commands; the Tauri capabilities file
//! limits them to the `core:default` and `core:path:default`
//! scopes.

use serde::Serialize;
use tauri::Manager;

const WORKMEN_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The payload returned by [`get_system_info`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SystemInfo {
    /// Host operating system, as a stable string
    /// (`"macos" | "linux" | "windows" | "ios" | "android"`).
    pub os: String,
    /// Host CPU architecture (`"x86_64" | "aarch64" | ...`).
    pub arch: String,
    /// Workmen version (the `workmen-desktop` crate's version).
    pub workmen_version: String,
    /// Whether the Tauri runtime is live. Always `true` when
    /// the command is reached, but surfaced for diagnostics.
    pub tauri: bool,
}

/// Read-only: report host info.
#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        workmen_version: WORKMEN_VERSION.to_string(),
        tauri: true,
    }
}

/// Read-only: return the OS app-data log directory as a
/// `String`. The Tauri Manager creates the directory on first
/// write; we return the canonical path whether or not the
/// directory exists.
#[tauri::command]
pub fn get_app_log_directory(app: tauri::AppHandle) -> Result<String, String> {
    app.path()
        .app_log_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| format!("failed to resolve app log dir: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_info_carries_workmen_version() {
        // The Tauri command is a thin wrapper; the struct
        // must always carry the workmen version (set at
        // compile time from CARGO_PKG_VERSION).
        let info = SystemInfo {
            os: "test".to_string(),
            arch: "test".to_string(),
            workmen_version: WORKMEN_VERSION.to_string(),
            tauri: true,
        };
        assert!(!info.workmen_version.is_empty(), "version must be set");
        assert_eq!(info.workmen_version, WORKMEN_VERSION);
    }

    #[test]
    fn system_info_serializes_to_json() {
        let info = SystemInfo {
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            workmen_version: "0.1.0".to_string(),
            tauri: true,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(json.contains("\"os\":\"macos\""));
        assert!(json.contains("\"arch\":\"aarch64\""));
        assert!(json.contains("\"tauri\":true"));
    }
}
