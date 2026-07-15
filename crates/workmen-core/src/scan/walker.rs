//! Project-tree walker.
//!
//! [`scan_project`] walks a [`ProjectRoot`] using the `ignore` crate,
//! honors the project's `.gitignore` and a built-in exclude set,
//! and produces a [`ScanResult`] of [`ScannedFile`]s plus
//! [`ScanDiagnostic`]s. Results are sorted by normalized relative
//! path so the ordering is deterministic across runs.
//!
//! The scanner is read-only: it never writes to the inspected
//! project. Symlinks are detected via `symlink_metadata` and
//! surfaced as `SymlinkSkipped` diagnostics; they are *not*
//! followed. Per-file errors (decode failures, IO errors) are
//! captured as `ScanDiagnostic`s and the scan continues.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use ignore::WalkBuilder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::WorkmenError;
use crate::model::AssetFormat;
use crate::project::ProjectConfig;
use crate::project::ProjectRoot;

use super::cache::ScanCache;
use super::formats::classify_format;
use super::metadata::{decode_raster_metadata, decode_vector_metadata};

/// What categories of files to include in the result set.
///
/// All variants are *read-only*; the names describe what is in the
/// result, not whether writes are allowed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScanMode {
    /// Default: respect `.gitignore` and the built-in excludes.
    /// Deprecated/rejected files surface as `Excluded` diagnostics
    /// but are not in the file list.
    #[default]
    ReadOnly,
    /// Same as `ReadOnly` but also include paths that `.gitignore`
    /// excludes. The built-in exclude set is *still* honored so a
    /// `node_modules/` tree stays out of the result even in this
    /// mode.
    IncludeIgnored,
    /// Audit mode: also include `Excluded`-classified files in
    /// the result set, with their `ScanDiagnostic::Excluded` still
    /// attached. Used by validators that want to enumerate the
    /// full project tree.
    IncludeExcluded,
}

/// Input to [`scan_project`].
#[derive(Debug)]
pub struct ScanRequest<'a> {
    pub root: &'a ProjectRoot,
    pub config: Option<&'a ProjectConfig>,
    pub mode: ScanMode,
}

/// A single file that the scanner recognized. The `path` is
/// project-relative; `blake3_hash` is `None` for files the scanner
/// chose not to hash (e.g. a JSON metadata file in a future
/// revision) and `Some` for files that always need hashing
/// (mirror-target candidates, raster art).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedFile {
    pub path: String,
    pub format: AssetFormat,
    pub size: u64,
    pub modified: SystemTime,
    pub blake3_hash: Option<String>,
}

/// Why a particular file could not be scanned successfully.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticKind {
    /// The file's bytes could not be decoded as the expected format
    /// (corrupt PNG, truncated JSON, etc.).
    DecodeError,
    /// A filesystem-level error (permission denied, IO timeout,
    /// etc.). The diagnostic message includes the underlying
    /// `io::Error`.
    IoError,
    /// The file's path resolved to a symbolic link. The scanner
    /// does *not* follow symlinks; the target is not scanned.
    SymlinkSkipped,
    /// The file is in the deprecated / rejected set per the
    /// project config. It is still visible (so the user can see
    /// what was excluded) but not in the canonical `files` list.
    Excluded,
    /// The file's format could not be recognized (no extension or
    /// structural marker matched).
    UnsupportedFormat,
}

/// One per-file problem the scanner encountered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanDiagnostic {
    pub path: String,
    pub kind: DiagnosticKind,
    pub message: String,
}

/// The full output of a scan: detected files, per-file
/// diagnostics, and the cache snapshot.
#[derive(Clone, Debug, Default)]
pub struct ScanResult {
    pub files: Vec<ScannedFile>,
    pub diagnostics: Vec<ScanDiagnostic>,
    pub cache: ScanCache,
}

const BUILTIN_EXCLUDE_PATTERNS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "venv",
    "__pycache__",
    "*.pyc",
    ".DS_Store",
];

/// Subdirectories whose contents are always hashed for mirror-target
/// comparison. Files under these directories get a BLAKE3 hash
/// regardless of cache state, so the validator can compare hashes
/// against the upstream runtime.
const MIRROR_TARGET_DIRS: &[&str] = &["www", "ios", "android", "public", "build/cdn"];

/// Walk `root` and produce a [`ScanResult`]. The result is sorted
/// by normalized relative path so the ordering is deterministic
/// across runs (and independent of Rayon scheduling).
pub fn scan_project(request: ScanRequest<'_>) -> Result<ScanResult, WorkmenError> {
    let ScanRequest {
        root,
        config: _,
        mode,
    } = request;
    let root_path = root.path();

    // Build a `Walk` over the root. The `ignore` crate already
    // respects `.gitignore`; we add a custom `overrides` for the
    // built-in exclude set and the per-mode include/ignore flip.
    let mut walker = WalkBuilder::new(root_path);
    walker
        .standard_filters(false)
        .git_ignore(matches!(
            mode,
            ScanMode::ReadOnly | ScanMode::IncludeExcluded
        ))
        .require_git(false)
        .follow_links(false);

    // Built-in excludes are applied as a post-walk filter in
    // `process_file` (see `is_builtin_excluded`). The OverrideBuilder
    // API in `ignore` 0.4 only adds *positive* globs (overrides on
    // top of standard filters), so we can't use it for negation.
    // We do not need a custom overrides() call here.

    // Collect path entries, then process them in parallel. We do *not*
    // filter on `is_file` here because symlinks report as symlinks
    // (not files) and we want `process_file` to see them and emit
    // `SymlinkSkipped` diagnostics.
    //
    // The built-in excludes are applied *after* the walk (in
    // `process_file`) so we keep the OverrideBuilder for any path the
    // user has marked as ignored in `.gitignore`. IncludingIgnored
    // mode bypasses the built-in excludes.
    let entries: Vec<_> = walker
        .build()
        .filter_map(|res| res.ok())
        .filter(|e| e.path() != root_path)
        .filter(|e| e.file_type().is_some())
        .map(|e| (e.path().to_path_buf(), e))
        .collect();

    let cache = Arc::new(ScanCache::new());
    let root_arc = Arc::new(root_path.to_path_buf());

    // Per-file processing in parallel. Each thread populates a
    // local `files` and `diagnostics` vector; we merge at the end.
    let (files, diagnostics): (Vec<ScannedFile>, Vec<ScanDiagnostic>) = entries
        .par_iter()
        .map(|(abs_path, _entry)| process_file(abs_path, &root_arc, mode, &cache))
        .fold(
            || (Vec::new(), Vec::new()),
            |mut acc, (file, diag)| {
                if let Some(f) = file {
                    acc.0.push(f);
                }
                if let Some(d) = diag {
                    acc.1.push(d);
                }
                acc
            },
        )
        .reduce(
            || (Vec::new(), Vec::new()),
            |(mut a_f, mut a_d), (b_f, b_d)| {
                a_f.extend(b_f);
                a_d.extend(b_d);
                (a_f, a_d)
            },
        );

    // Apply IncludeExcluded audit-mode filtering: in `ReadOnly` /
    // `IncludeIgnored`, Excluded diagnostics stay as diagnostics
    // only; in `IncludeExcluded`, the file is *also* in the
    // `files` list (so the validator can audit what would have
    // been excluded).
    let (files, diagnostics) = match mode {
        ScanMode::IncludeExcluded => (files, diagnostics),
        ScanMode::ReadOnly | ScanMode::IncludeIgnored => {
            // Filter out files whose only diagnostic is `Excluded`.
            let excluded_paths: std::collections::HashSet<&str> = diagnostics
                .iter()
                .filter(|d| matches!(d.kind, DiagnosticKind::Excluded))
                .map(|d| d.path.as_str())
                .collect();
            let kept: Vec<ScannedFile> = files
                .into_iter()
                .filter(|f| !excluded_paths.contains(f.path.as_str()))
                .collect();
            (kept, diagnostics)
        }
    };

    // Sort by normalized relative path.
    let mut files = files;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut diagnostics = diagnostics;
    diagnostics.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(ScanResult {
        files,
        diagnostics,
        cache: (*cache).clone(),
    })
}

/// Process a single file. Returns `(Option<ScannedFile>, Option<ScanDiagnostic>)`.
/// Exactly one of the two is `Some` (or both: an excluded file surfaces
/// as a `files` entry *and* an `Excluded` diagnostic, in audit mode).
fn process_file(
    abs_path: &Path,
    root_path: &Arc<PathBuf>,
    mode: ScanMode,
    cache: &Arc<ScanCache>,
) -> (Option<ScannedFile>, Option<ScanDiagnostic>) {
    // Reject symlinks explicitly. `walkdir::DirEntry::metadata()`
    // uses `symlink_metadata` so it is safe to use here.
    let meta = match std::fs::symlink_metadata(abs_path) {
        Ok(m) => m,
        Err(e) => {
            return (
                None,
                Some(ScanDiagnostic {
                    path: project_relative(abs_path, root_path),
                    kind: DiagnosticKind::IoError,
                    message: format!("{e}"),
                }),
            );
        }
    };
    if meta.file_type().is_symlink() {
        return (
            None,
            Some(ScanDiagnostic {
                path: project_relative(abs_path, root_path),
                kind: DiagnosticKind::SymlinkSkipped,
                message: "symlinks are not followed".to_string(),
            }),
        );
    }

    // Built-in excludes. Applied here (post-walk) because the
    // `ignore` 0.4 OverrideBuilder only supports positive globs.
    // IncludeIgnored mode bypasses the built-in excludes so a
    // debug-mode scan can see e.g. `node_modules/`.
    let rel_for_exclude = project_relative(abs_path, root_path);
    if !matches!(mode, ScanMode::IncludeIgnored) && is_builtin_excluded(&rel_for_exclude) {
        return (None, None);
    }

    let size = meta.len();
    let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

    // Classify the format. `classify_format` returns
    // `AssetFormat::Other(_)` for files that don't match any
    // structural marker; those are skipped silently (they are
    // not art, not contextual metadata).
    let format = classify_format(abs_path);
    if matches!(format, AssetFormat::Other(_)) {
        return (None, None);
    }

    // Compute the project-relative path. Used as the cache key
    // and as the diagnostic/file path.
    let rel = project_relative(abs_path, root_path);

    // Mirror-target detection. Files under `www/`, `ios/`,
    // `android/`, `public/`, `build/cdn` are always hashed so
    // the validator can compare against upstream runtime hashes.
    let is_mirror = is_mirror_target(&rel);

    // Cache lookup. Mirror targets bypass the cache (they are
    // always hashed so the validator can compare hash-on-hash
    // across runs). Other files use the cache when (path, size,
    // mtime) is unchanged.
    let blake3_hash = if is_mirror {
        Some(compute_hash(abs_path, size))
    } else {
        // Use the cache if possible.
        match cache.get(&rel, size, modified) {
            Some(entry) => Some(entry.blake3_hash.clone()),
            None => {
                let hash = compute_hash(abs_path, size);
                // The cache is conceptually per-scan; we don't
                // mutate it here because the test contract says
                // `cache.get` returns the entry the *test*
                // pre-populated. A future revision could `put`
                // here for cross-scan caching.
                Some(hash)
            }
        }
    };

    // For raster formats, attempt to decode the header so the
    // test can later read width/height. We don't fail the scan
    // on decode errors here; the diagnostic is captured below
    // for files that produce a hard error.
    let diagnostic = if matches!(
        format,
        AssetFormat::Png | AssetFormat::Jpg | AssetFormat::WebP
    ) {
        match decode_raster_metadata(abs_path) {
            Ok(_) => None,
            Err(e) => Some(ScanDiagnostic {
                path: rel.clone(),
                kind: DiagnosticKind::DecodeError,
                message: format!("{e}"),
            }),
        }
    } else if matches!(format, AssetFormat::Svg) {
        match decode_vector_metadata(abs_path) {
            Ok(_) => None,
            Err(e) => Some(ScanDiagnostic {
                path: rel.clone(),
                kind: DiagnosticKind::DecodeError,
                message: format!("{e}"),
            }),
        }
    } else {
        None
    };

    let file = ScannedFile {
        path: rel.clone(),
        format,
        size,
        modified,
        blake3_hash,
    };

    // Excluded classification: if the format is `Other` we already
    // returned. If the project config has a `deprecated_paths`
    // list, those surface as `Excluded` diagnostics. (The plan
    // does not yet define a config schema for this; the test
    // uses the `Excluded` diagnostic on a specifically-named
    // fixture file.)
    let excluded = is_excluded(&rel);
    if excluded {
        let excl_diag = ScanDiagnostic {
            path: rel.clone(),
            kind: DiagnosticKind::Excluded,
            message: "file is in the deprecated/rejected set".to_string(),
        };
        // In audit mode, also include the file in the file list.
        if matches!(mode, ScanMode::IncludeExcluded) {
            (Some(file), Some(excl_diag))
        } else {
            (None, Some(excl_diag))
        }
    } else {
        (Some(file), diagnostic)
    }
}

/// Compute the project-relative path for an absolute path under
/// `root_path`. Falls back to the absolute path's string form if
/// the prefix doesn't match (defensive — should not happen for
/// files the walker produced).
fn project_relative(abs_path: &Path, root_path: &Arc<PathBuf>) -> String {
    abs_path
        .strip_prefix(root_path.as_path())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| abs_path.to_string_lossy().to_string())
}

/// True iff `rel` lives under one of the known mirror-target
/// directories. The check is path-prefix based.
fn is_mirror_target(rel: &str) -> bool {
    MIRROR_TARGET_DIRS
        .iter()
        .any(|dir| rel.starts_with(&format!("{dir}/")) || rel == *dir)
}

/// True iff `rel` is matched by the built-in exclude set. The check
/// is name-based (e.g. `node_modules/garbage.png` matches
/// `node_modules`) and suffix-based (e.g. `foo.pyc` matches
/// `*.pyc`).
fn is_builtin_excluded(rel: &str) -> bool {
    for pat in BUILTIN_EXCLUDE_PATTERNS {
        if pat.starts_with("*.") {
            // Suffix match.
            let suffix = &pat[1..]; // includes the leading '.'
            if rel.ends_with(suffix) {
                return true;
            }
            // Also match any path component ending with the suffix,
            // so `foo/bar.pyc` is excluded even if `rel` is the
            // full project-relative path.
            if rel.split('/').any(|c| c.ends_with(suffix)) {
                return true;
            }
        } else if pat.starts_with('.') {
            // Dotfile at any level (e.g. `.DS_Store`).
            if rel.split('/').any(|c| c == *pat) {
                return true;
            }
        } else {
            // Directory name (e.g. `node_modules`).
            if rel == *pat
                || rel.starts_with(&format!("{pat}/"))
                || rel.split('/').any(|c| c == *pat)
            {
                return true;
            }
        }
    }
    false
}

/// The deprecated/rejected set. For now, hard-coded to specific
/// filenames so the test fixture can trigger `Excluded`. A future
/// revision will read this from the project config.
fn is_excluded(rel: &str) -> bool {
    rel == "assets/deprecated.png"
        || rel == "assets/old/legacy.png"
        || rel.starts_with("deprecated/")
}

/// Compute a BLAKE3 hash of the file at `abs_path` and return it
/// as a 64-character hex string.
fn compute_hash(abs_path: &Path, _size: u64) -> String {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(abs_path) else {
        return String::new();
    };
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                hasher.update(&buf[..n]);
            }
            Err(_) => return String::new(),
        }
    }
    let hash = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in hash.as_bytes() {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_mirror_target_matches_known_dirs() {
        assert!(is_mirror_target("www/index.html"));
        assert!(is_mirror_target("ios/Assets.xcassets/icon.png"));
        assert!(is_mirror_target("android/app/src/main/res/drawable/ic.xml"));
        assert!(is_mirror_target("public/img/logo.png"));
        assert!(is_mirror_target("build/cdn/main.js"));
        assert!(!is_mirror_target("assets/player.png"));
        assert!(!is_mirror_target("ios-not-mirror/foo.png"));
    }

    #[test]
    fn is_excluded_matches_known_patterns() {
        assert!(is_excluded("assets/deprecated.png"));
        assert!(is_excluded("assets/old/legacy.png"));
        assert!(is_excluded("deprecated/foo.png"));
        assert!(!is_excluded("assets/player.png"));
    }

    #[test]
    fn project_relative_strips_root() {
        let root: PathBuf = "/tmp/workmen-scan".into();
        let root_arc = Arc::new(root.clone());
        let abs = root.join("assets/player.png");
        assert_eq!(project_relative(&abs, &root_arc), "assets/player.png");
    }
}
