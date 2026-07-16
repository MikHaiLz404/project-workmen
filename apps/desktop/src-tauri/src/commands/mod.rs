//! Tauri command surface. Each module here exposes typed
//! commands that the React shell invokes through
//! `@workmen/contracts` (see `packages/contracts/src/generated.ts`).

pub mod project;
pub mod system;

pub use project::{CancelRegistry, ProjectSnapshot, ScanProgress, new_cancel_registry};
