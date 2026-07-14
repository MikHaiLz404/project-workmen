//! Project root, config, and initializer.
//!
//! `ProjectRoot::discover` finds the project boundary (`.git` or `.workmen`).
//! `ProjectConfig::load` reads `.workmen/project.yaml` and the spec files.
//! `ProjectInitializer::{preview, commit}` initializes an un-initialized
//! project. All paths are project-relative.

mod config;
mod init;
mod migrate;
mod root;

pub use config::{InitPreview, ProjectConfig, ProjectYaml, SpecFile};
pub use init::ProjectInitializer;
pub use migrate::migrate;
pub use root::{ProjectRoot, RootMarker};
