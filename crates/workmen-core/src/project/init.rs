//! Project initialization.
//!
//! Initialization of a Game Project is a *two-step* operation:
//!
//! 1. [`ProjectInitializer::preview`] computes the proposed state
//!    (paths, default project.yaml body) **without writing anything**.
//! 2. [`ProjectInitializer::commit`] atomically applies the preview,
//!    but only when the caller passes `confirmed = true`.
//!
//! The atomic write uses a sibling staging directory and a final
//! `rename` so a half-written `.workmen/` is never observable on disk.
//! On any failure the staging directory is removed and the original
//! project tree is left untouched.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::WorkmenError;
use crate::model::ProfileId;

use super::config::{InitPreview, PROJECT_SCHEMA_VERSION, ProjectConfig, ProjectYaml};
use super::root::ProjectRoot;

/// Safe project initializer.
///
/// Methods are exposed as static functions; the type itself carries no
/// state.
pub struct ProjectInitializer;

impl ProjectInitializer {
    /// Compute the proposed initial state for `root`.
    ///
    /// # Errors
    ///
    /// * [`WorkmenError::Config`] — `.workmen/` already exists in
    ///   `root`. The message names the offending path.
    /// * [`WorkmenError::Io`] — filesystem inspection of `root` fails.
    pub fn preview(root: &ProjectRoot) -> Result<InitPreview, WorkmenError> {
        let root_path = root.path();
        let dot_workmen = root_path.join(".workmen");
        let relative = |abs: &Path| -> PathBuf {
            abs.strip_prefix(root_path)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| abs.to_path_buf())
        };

        if dot_workmen.exists() {
            return Err(WorkmenError::config(
                format!(
                    "workmen config already initialized at {}",
                    relative(&dot_workmen).display()
                ),
                &relative(&dot_workmen),
            ));
        }

        let project_yaml = default_project_yaml(root_path);
        let project_yaml_bytes = serde_yaml::to_string(&project_yaml).map_err(|e| {
            WorkmenError::config(
                format!("failed to serialize default ProjectYaml: {e}"),
                &relative(&dot_workmen.join("project.yaml")),
            )
        })?;

        Ok(InitPreview {
            project_yaml_path: dot_workmen.join("project.yaml"),
            specs_dir: dot_workmen.join("specs"),
            project_yaml_bytes,
        })
    }

    /// Atomically write the previewed state to disk.
    ///
    /// * When `confirmed` is `false`, returns [`WorkmenError::Config`]
    ///   and writes nothing.
    /// * When `confirmed` is `true`, stages the new `.workmen/`
    ///   directory at a sibling temporary location, then renames it
    ///   into place. The rename is on the same filesystem as the
    ///   project root (the staging dir is a sibling), so a successful
    ///   rename is atomic on POSIX and Windows.
    ///
    /// On any failure during staging, the staging directory is removed
    /// and the original project tree is left untouched. On success,
    /// the freshly written config is reloaded and returned so the
    /// caller does not need to re-parse what it just wrote.
    pub fn commit(preview: InitPreview, confirmed: bool) -> Result<ProjectConfig, WorkmenError> {
        if !confirmed {
            return Err(WorkmenError::config(
                "init requires --confirm",
                Path::new(".workmen/project.yaml"),
            ));
        }

        // The staging directory is a *sibling* of the target `.workmen/`
        // so the final rename stays on the same filesystem.
        let final_dir = preview
            .project_yaml_path
            .parent()
            .expect("project_yaml_path must have a parent directory");
        let staging = sibling_staging_dir(final_dir);

        // Make sure we don't leave a stale staging directory around.
        // `create_dir` (not `create_dir_all`) so a leftover from a
        // crashed previous run is surfaced as an error rather than
        // silently reused.
        std::fs::create_dir(&staging).map_err(|e| WorkmenError::io(&staging, e))?;

        // Stage the file. If anything fails, clean up the staging dir
        // before returning the error.
        let staging_project_yaml = staging.join("project.yaml");
        if let Err(e) = std::fs::write(&staging_project_yaml, &preview.project_yaml_bytes) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(WorkmenError::io(&staging_project_yaml, e));
        }
        let staging_specs = staging.join("specs");
        if let Err(e) = std::fs::create_dir(&staging_specs) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(WorkmenError::io(&staging_specs, e));
        }

        // Rename staging → final. The staging directory is a sibling
        // of `final_dir`, so `rename` stays on the same filesystem and
        // is atomic on POSIX / Windows.
        if let Err(e) = std::fs::rename(&staging, final_dir) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(WorkmenError::io(final_dir, e));
        }

        // Reload via ProjectConfig::load so callers always get the
        // parsed form of what was actually written.
        let root = ProjectRoot {
            path: final_dir
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| final_dir.to_path_buf()),
            marker: super::root::RootMarker::WorkmenDir,
        };
        ProjectConfig::load(&root)?.ok_or_else(|| {
            WorkmenError::config(
                "freshly-written .workmen/project.yaml could not be reloaded",
                Path::new(".workmen/project.yaml"),
            )
        })
    }
}

/// Construct a unique staging directory next to `final_dir`.
///
/// The format is `.workmen.init.<pid>.<nanos>` so concurrent
/// initializers do not collide.
fn sibling_staging_dir(final_dir: &Path) -> PathBuf {
    let parent = final_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let stem = final_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(".workmen");
    parent.join(format!("{stem}.init.{pid}.{nanos}"))
}

/// Slugify a directory name into a safe project id.
///
/// Lowercase ASCII, dashes for any non-alphanumeric run, leading/trailing
/// dashes trimmed. Empty input falls back to `"untitled"`.
fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_was_dash = true;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            for low in ch.to_lowercase() {
                out.push(low);
            }
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed
    }
}

/// Build a fresh [`ProjectYaml`] for `root_path`. The id is derived
/// from the directory's basename slugified, the active profile is
/// `"default"`, and the profiles list contains that same id.
fn default_project_yaml(root_path: &Path) -> ProjectYaml {
    let id_slug = root_path
        .file_name()
        .and_then(|s| s.to_str())
        .map(slugify)
        .unwrap_or_else(|| "untitled".to_string());
    let active = ProfileId("default".to_string());
    ProjectYaml {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: id_slug,
        active_profile: active.clone(),
        profiles: vec![active],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_tempdir(label: &str) -> PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("workmen-init-{label}-{pid}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("create tempdir");
        dir
    }

    fn remove_tempdir(path: &Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn preview_returns_paths_and_bytes_without_writing() {
        let tmp = fresh_tempdir("preview");
        let root = ProjectRoot::discover(&tmp).unwrap();
        let preview = ProjectInitializer::preview(&root).unwrap();
        assert_eq!(
            preview.project_yaml_path,
            root.path().join(".workmen/project.yaml")
        );
        assert_eq!(preview.specs_dir, root.path().join(".workmen/specs"));
        assert!(!preview.project_yaml_bytes.is_empty());
        assert!(!root.path().join(".workmen").exists());
        remove_tempdir(&tmp);
    }

    #[test]
    fn commit_without_confirm_returns_config_error() {
        let tmp = fresh_tempdir("no-confirm");
        let root = ProjectRoot::discover(&tmp).unwrap();
        let preview = ProjectInitializer::preview(&root).unwrap();
        let err = ProjectInitializer::commit(preview, false).unwrap_err();
        match err {
            WorkmenError::Config { message, .. } => {
                assert!(message.contains("--confirm"), "got {message:?}");
            }
            other => panic!("expected Config error, got {other:?}"),
        }
        assert!(!root.path().join(".workmen").exists());
        remove_tempdir(&tmp);
    }

    #[test]
    fn commit_with_confirm_writes_project_yaml_and_empty_specs() {
        let tmp = fresh_tempdir("with-confirm");
        let root = ProjectRoot::discover(&tmp).unwrap();
        let preview = ProjectInitializer::preview(&root).unwrap();
        let cfg = ProjectInitializer::commit(preview, true).unwrap();
        let dot_workmen = root.path().join(".workmen");
        assert!(dot_workmen.is_dir());
        assert!(dot_workmen.join("project.yaml").is_file());
        assert!(dot_workmen.join("specs").is_dir());
        let specs: Vec<_> = std::fs::read_dir(dot_workmen.join("specs"))
            .unwrap()
            .collect();
        assert!(specs.is_empty(), "specs/ must start empty, got {specs:?}");
        assert_eq!(cfg.project_yaml.schema_version, PROJECT_SCHEMA_VERSION);
        remove_tempdir(&tmp);
    }

    #[test]
    fn double_init_attempt_returns_config_error() {
        let tmp = fresh_tempdir("double-init");
        let root = ProjectRoot::discover(&tmp).unwrap();
        let preview = ProjectInitializer::preview(&root).unwrap();
        let _ = ProjectInitializer::commit(preview, true).unwrap();

        // Second preview must fail because .workmen/ now exists.
        let err = ProjectInitializer::preview(&root).unwrap_err();
        match err {
            WorkmenError::Config { message, .. } => {
                assert!(message.contains("already initialized"), "got {message:?}");
            }
            other => panic!("expected Config error, got {other:?}"),
        }
        remove_tempdir(&tmp);
    }

    #[test]
    fn slugify_normalizes_directory_names() {
        assert_eq!(slugify("Night Market Merge"), "night-market-merge");
        assert_eq!(slugify("  My Game!!! "), "my-game");
        assert_eq!(slugify("---"), "untitled");
        assert_eq!(slugify("snake_case"), "snake-case");
    }

    #[test]
    fn default_project_yaml_uses_slug_of_directory_name() {
        let yaml = default_project_yaml(Path::new("/tmp/Night Market Merge"));
        assert_eq!(yaml.id, "night-market-merge");
        assert_eq!(yaml.active_profile, ProfileId("default".to_string()));
        assert_eq!(yaml.profiles, vec![ProfileId("default".to_string())]);
    }
}
