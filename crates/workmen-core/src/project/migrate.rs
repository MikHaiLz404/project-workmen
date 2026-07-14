//! Project configuration migration seam.
//!
//! This module is the single chokepoint for evolving `.workmen/project.yaml`
//! across schema versions. Today the only recognized version is `1`, so:
//!
//! * `migrate(yaml, 1, 1)` is the identity transform.
//! * `migrate(yaml, 1, 2)` returns [`WorkmenError::Config`] explaining
//!   that the migration has not been implemented yet — the gap is
//!   intentional and documented so a future PR can land it without
//!   touching the [`ProjectConfig::load`](super::config::ProjectConfig::load)
//!   surface.
//! * Any `current_version` other than `1` returns [`WorkmenError::Config`]
//!   reporting the unsupported version.
//!
//! Future tasks will add real migration steps here without changing the
//! public function signature.

use crate::WorkmenError;

/// Migrate a project YAML from `current_version` to `target_version`.
///
/// See module docs for the current support matrix.
pub fn migrate(
    yaml: &str,
    current_version: u32,
    target_version: u32,
) -> Result<String, WorkmenError> {
    if current_version != super::config::PROJECT_SCHEMA_VERSION {
        return Err(WorkmenError::config(
            format!("unsupported project schema version {current_version}"),
            std::path::Path::new(".workmen/project.yaml"),
        ));
    }
    if current_version == target_version {
        return Ok(yaml.to_string());
    }
    Err(WorkmenError::config(
        format!(
            "migration from v{current_version} to v{target_version} is not implemented yet; \
             please open a profile PR"
        ),
        std::path::Path::new(".workmen/project.yaml"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_v1_to_v1_returns_input_unchanged() {
        let input = "schemaVersion: 1\nid: x\n";
        let out = migrate(input, 1, 1).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn v1_to_v2_returns_not_implemented_error() {
        let err = migrate("schemaVersion: 1\n", 1, 2).unwrap_err();
        match err {
            WorkmenError::Config { message, .. } => {
                assert!(
                    message.contains("not implemented"),
                    "expected 'not implemented' message, got {message:?}"
                );
                assert!(message.contains("v1"), "must mention v1, got {message:?}");
                assert!(message.contains("v2"), "must mention v2, got {message:?}");
            }
            other => panic!("expected Config error, got {other:?}"),
        }
    }

    #[test]
    fn unsupported_current_version_returns_config_error() {
        let err = migrate("schemaVersion: 99\n", 99, 1).unwrap_err();
        match err {
            WorkmenError::Config { message, .. } => {
                assert!(message.contains("unsupported"), "got {message:?}");
                assert!(
                    message.contains("99"),
                    "must mention version, got {message:?}"
                );
            }
            other => panic!("expected Config error, got {other:?}"),
        }
    }
}
