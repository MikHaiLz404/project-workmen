use thiserror::Error;

use std::path::{Path, PathBuf};

/// Top-level error type for Workmen.
///
/// `WorkmenError` is the single error surface used across `workmen-core` and
/// surfaced by `workmen-cli`. It carries enough context (a path, a message,
/// or a wrapped `std::io::Error`) for callers and operators to diagnose a
/// failure without having to dig through generic `Box<dyn Error>` payloads.
///
/// Paths stored in this type are expected to be **project-relative**. Callers
/// must convert absolute paths to relative ones before constructing a
/// `WorkmenError`; use [`WorkmenError::has_absolute_path`] to assert that
/// invariant in tests.
#[derive(Debug, Error)]
pub enum WorkmenError {
    #[error("config error: {message} (path: {})", path.display())]
    Config { message: String, path: PathBuf },

    #[error("io error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("decode error at {}: {message}", path.display())]
    Decode { path: PathBuf, message: String },

    #[error("validation error: {message}")]
    Validation { message: String },

    #[error("internal error: {message}")]
    Internal { message: String },
}

impl WorkmenError {
    /// Construct a [`WorkmenError::Config`] from a message and a path.
    pub fn config(message: impl Into<String>, path: &Path) -> Self {
        Self::Config {
            message: message.into(),
            path: path.to_path_buf(),
        }
    }

    /// Construct an [`WorkmenError::Io`] from a path and a `std::io::Error`.
    pub fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }

    /// Construct a [`WorkmenError::Decode`] from a path and a message.
    pub fn decode(path: &Path, message: impl Into<String>) -> Self {
        Self::Decode {
            path: path.to_path_buf(),
            message: message.into(),
        }
    }

    /// Construct a [`WorkmenError::Validation`] from a message.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Construct an [`WorkmenError::Internal`] from a message.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Returns true if any contained path was absolute. Callers may use this
    /// to gate whether a serialized form is allowed to ship to logs/reports.
    pub fn has_absolute_path(&self) -> bool {
        let p = match self {
            Self::Config { path, .. } | Self::Io { path, .. } | Self::Decode { path, .. } => path,
            Self::Validation { .. } | Self::Internal { .. } => return false,
        };
        p.is_absolute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_constructor_sets_message_and_path() {
        let err = WorkmenError::config("bad yaml", Path::new(".workmen/profiles/foo.yaml"));
        match &err {
            WorkmenError::Config { message, path } => {
                assert_eq!(message, "bad yaml");
                assert_eq!(path, Path::new(".workmen/profiles/foo.yaml"));
            }
            other => panic!("expected Config, got {other:?}"),
        }
    }

    #[test]
    fn io_constructor_wraps_source_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err = WorkmenError::io(Path::new("assets/missing.png"), io);
        match &err {
            WorkmenError::Io { path, source } => {
                assert_eq!(path, Path::new("assets/missing.png"));
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn decode_constructor_sets_path_and_message() {
        let err = WorkmenError::decode(Path::new("bad.png"), "unsupported format");
        match &err {
            WorkmenError::Decode { path, message } => {
                assert_eq!(path, Path::new("bad.png"));
                assert_eq!(message, "unsupported format");
            }
            other => panic!("expected Decode, got {other:?}"),
        }
    }

    #[test]
    fn validation_and_internal_constructors_have_no_path() {
        let v = WorkmenError::validation("profile mismatch");
        assert!(matches!(v, WorkmenError::Validation { .. }));
        let i = WorkmenError::internal("not implemented yet");
        assert!(matches!(i, WorkmenError::Internal { .. }));
    }

    #[test]
    fn display_includes_the_path() {
        let err = WorkmenError::config("bad yaml", Path::new(".workmen/profiles/foo.yaml"));
        let s = err.to_string();
        assert!(s.contains("bad yaml"), "missing message in {s:?}");
        assert!(
            s.contains(".workmen/profiles/foo.yaml"),
            "missing path in {s:?}"
        );
    }

    #[test]
    fn display_includes_io_source_text() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = WorkmenError::io(Path::new("assets/x.png"), io);
        let s = err.to_string();
        assert!(s.contains("assets/x.png"));
        assert!(s.contains("denied"));
    }

    #[test]
    fn has_absolute_path_is_false_for_relative_paths() {
        let cfg = WorkmenError::config("x", Path::new("relative/path.yaml"));
        assert!(!cfg.has_absolute_path());
        let io = WorkmenError::io(Path::new("relative/x.png"), std::io::Error::other("x"));
        assert!(!io.has_absolute_path());
        let dec = WorkmenError::decode(Path::new("relative/bad.png"), "bad");
        assert!(!dec.has_absolute_path());
    }

    #[test]
    fn has_absolute_path_is_true_for_absolute_paths() {
        let abs = std::path::PathBuf::from("/etc/passwd");
        let cfg = WorkmenError::config("x", &abs);
        assert!(cfg.has_absolute_path());
        let io = WorkmenError::io(&abs, std::io::Error::other("x"));
        assert!(io.has_absolute_path());
        let dec = WorkmenError::decode(&abs, "bad");
        assert!(dec.has_absolute_path());
    }

    #[test]
    fn has_absolute_path_is_false_for_pathless_variants() {
        assert!(!WorkmenError::validation("v").has_absolute_path());
        assert!(!WorkmenError::internal("i").has_absolute_path());
    }

    #[test]
    fn implements_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<WorkmenError>();
    }
}
