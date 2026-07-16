//! Project root discovery.
//!
//! [`ProjectRoot::discover`] walks upward from a starting path looking
//! for the `.git` or `.workmen` marker directory. Whichever it finds
//! first defines the project root (the *parent* of the marker).
//! If neither marker is present at any level, the supplied directory
//! itself is reported as the root with a [`RootMarker::SuppliedDir`]
//! marker so callers can distinguish "user pointed at a project" from
//! "we walked upward and found a project boundary".
//!
//! Symlinked markers are explicitly rejected: a `.git` or `.workmen`
//! entry that is itself a symbolic link is treated as *not present*,
//! matching the design rule that "symlinks are not followed by default".

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::WorkmenError;

/// The marker that identified a [`ProjectRoot`].
///
/// `GitDir` is preferred over `WorkmenDir` when both exist at the same
/// level — the design treats the git boundary as the authoritative
/// project edge when available.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RootMarker {
    /// The root was identified by an ancestor `.git` directory.
    GitDir,
    /// The root was identified by an ancestor `.workmen` directory.
    WorkmenDir,
    /// No marker was found; the supplied directory itself is the root.
    SuppliedDir,
}

/// A discovered project root.
///
/// `path` is always an absolute path; it is the anchor used to resolve
/// project-relative paths everywhere else in Workmen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectRoot {
    pub(crate) path: PathBuf,
    pub(crate) marker: RootMarker,
}

impl ProjectRoot {
    /// Walk upward from `start` looking for a `.git` or `.workmen`
    /// directory. Returns the parent of the marker, or `start` itself
    /// if neither marker is found.
    ///
    /// # Errors
    ///
    /// * [`WorkmenError::Io`] — `start` does not exist (or cannot be
    ///   resolved for any other reason).
    ///
    /// Symlinked `.git` or `.workmen` entries are ignored.
    pub fn discover(start: &Path) -> Result<Self, WorkmenError> {
        // Resolve `start` to an absolute path. `canonicalize` requires
        // the path to exist, so this also doubles as the "does it
        // exist?" check.
        let absolute = start
            .canonicalize()
            .map_err(|e| WorkmenError::io(start, e))?;

        // Walk upward: check the start itself first, then its parent,
        // and so on, until we reach the filesystem root.
        let mut cursor: Option<&Path> = Some(absolute.as_path());
        while let Some(dir) = cursor {
            // `.git` wins over `.workmen` at the same level, so check it
            // first.
            if is_real_dir(&dir.join(".git")) {
                // `dir` is the directory that *contains* `.git` — i.e.
                // the project root. The marker itself is `.git`, so the
                // root IS `dir`.
                return Ok(Self {
                    path: dir.to_path_buf(),
                    marker: RootMarker::GitDir,
                });
            }
            if is_real_dir(&dir.join(".workmen")) {
                // Same as above: `dir` is the project root.
                return Ok(Self {
                    path: dir.to_path_buf(),
                    marker: RootMarker::WorkmenDir,
                });
            }
            cursor = dir.parent();
        }

        // No marker found — the supplied directory is the root.
        Ok(Self {
            path: absolute,
            marker: RootMarker::SuppliedDir,
        })
    }

    /// The absolute path of the project root.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The marker that identified the root.
    pub fn marker(&self) -> RootMarker {
        self.marker
    }
}

/// Returns `true` if `path` exists, is a directory, and is not a
/// symbolic link. Uses [`std::fs::metadata`] (which follows symlinks)
/// followed by [`std::fs::symlink_metadata`] (which does not) so we
/// can reject symlinked markers explicitly.
fn is_real_dir(path: &Path) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if meta.file_type().is_symlink() {
        return false;
    }
    meta.is_dir()
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
        let dir = std::env::temp_dir().join(format!("workmen-root-{label}-{pid}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("create tempdir");
        dir
    }

    fn remove_tempdir(path: &Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn marker_priority_git_beats_workmen() {
        let tmp = fresh_tempdir("priority");
        std::fs::create_dir_all(tmp.join(".git")).unwrap();
        std::fs::create_dir_all(tmp.join(".workmen")).unwrap();

        let root = ProjectRoot::discover(&tmp).unwrap();
        assert_eq!(root.marker(), RootMarker::GitDir);

        remove_tempdir(&tmp);
    }

    #[test]
    fn supplied_dir_fallback_when_no_marker_found() {
        let tmp = fresh_tempdir("fallback");
        let root = ProjectRoot::discover(&tmp).unwrap();
        assert_eq!(root.marker(), RootMarker::SuppliedDir);
        // The discovered path must equal the supplied (absolute) path.
        let expected = std::fs::canonicalize(&tmp).unwrap();
        assert_eq!(root.path(), expected.as_path());

        remove_tempdir(&tmp);
    }

    #[test]
    fn finds_existing_git_marker_via_walk() {
        // Point at the workmen workspace itself, which has a real `.git`.
        // The discovered root must be that workspace root and the marker
        // must be `GitDir`.
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap()
            .to_path_buf();
        // Make a nested directory inside `target/` that is guaranteed
        // to walk up to the workspace's `.git` marker.
        let nested = workspace.join("target").join("__workmen_root_marker");
        std::fs::create_dir_all(&nested).unwrap();

        let root = ProjectRoot::discover(&nested).unwrap();
        assert_eq!(root.marker(), RootMarker::GitDir);
        assert!(
            root.path().starts_with(&workspace),
            "discovered root {:?} must be inside workspace {:?}",
            root.path(),
            workspace
        );

        let _ = std::fs::remove_dir_all(&nested);
    }

    #[test]
    fn finds_workmen_marker_when_no_git_present() {
        let tmp = fresh_tempdir("dot-workmen");
        std::fs::create_dir_all(tmp.join(".workmen")).unwrap();
        let nested = tmp.join("src").join("ui");
        std::fs::create_dir_all(&nested).unwrap();

        let root = ProjectRoot::discover(&nested).unwrap();
        assert_eq!(root.marker(), RootMarker::WorkmenDir);

        remove_tempdir(&tmp);
    }

    #[test]
    fn rejects_symlinked_marker() {
        // Build a temp dir that has a `.workmen` symlink pointing at a
        // real directory. The marker must be treated as absent, so the
        // root falls back to SuppliedDir.
        let tmp = fresh_tempdir("symlinked");
        let real = fresh_tempdir("symlinked-real");
        std::fs::create_dir_all(&real).unwrap();
        std::os::unix::fs::symlink(&real, tmp.join(".workmen")).unwrap();

        let root = ProjectRoot::discover(&tmp).unwrap();
        assert_eq!(root.marker(), RootMarker::SuppliedDir);

        remove_tempdir(&tmp);
        remove_tempdir(&real);
    }

    #[test]
    fn missing_start_returns_io_error() {
        let missing = std::env::temp_dir().join("workmen-root-missing-NEVER-EXIST-zzz");
        let _ = std::fs::remove_dir_all(&missing);
        let err = ProjectRoot::discover(&missing).unwrap_err();
        assert!(matches!(err, WorkmenError::Io { .. }));
    }
}
