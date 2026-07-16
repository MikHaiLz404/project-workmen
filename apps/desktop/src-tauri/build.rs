// Build script for the Workmen Tauri 2 desktop host.
//
// `tauri_build::build()` reads tauri.conf.json and emits the
// `OUT_DIR` bindings the `tauri::generate_context!` macro needs.

fn main() {
    tauri_build::build()
}
