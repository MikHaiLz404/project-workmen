//! Project configuration loading.
//!
//! [`ProjectConfig::load`] reads `.workmen/project.yaml` and the profile
//! spec files under `.workmen/specs/`. Both files are parsed through
//! the typed contracts in [`crate::model`] — the T2 schemas remain the
//! on-disk format of record, so any drift will trip the schema-drift gate.
//!
//! Paths reported in [`WorkmenError`] are project-relative, never
//! absolute, so logs and reports can serialize them safely.

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::WorkmenError;
use crate::model::{Profile, ProfileId};

use super::root::ProjectRoot;

/// The on-disk schema version of a project configuration. Bumped
/// alongside any backward-incompatible change to [`ProjectYaml`].
pub const PROJECT_SCHEMA_VERSION: u32 = 1;

/// A loaded project configuration: the top-level `project.yaml` and the
/// profile specs it references.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectConfig {
    pub project_yaml: ProjectYaml,
    pub specs: Vec<SpecFile>,
}

/// The top-level project configuration file.
///
/// `ProjectYaml` is a thin wrapper around the project-level metadata
/// (id, active profile, listed profile ids) and leaves the actual
/// [`Profile`] contracts to the spec files under `.workmen/specs/`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectYaml {
    #[serde(
        rename = "schemaVersion",
        deserialize_with = "deserialize_project_schema_version"
    )]
    pub schema_version: u32,
    pub id: String,
    pub active_profile: ProfileId,
    pub profiles: Vec<ProfileId>,
}

fn deserialize_project_schema_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u32::deserialize(deserializer)?;
    if value == PROJECT_SCHEMA_VERSION {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "unsupported project schema version {value}; Workmen only understands version {PROJECT_SCHEMA_VERSION}"
        )))
    }
}

/// A profile loaded from `.workmen/specs/<name>.yaml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecFile {
    /// Project-relative path of the spec file within `.workmen/specs/`.
    pub path: String,
    pub profile: Profile,
}

/// The proposed initial state for a freshly-initialized project.
///
/// Returned by [`super::init::ProjectInitializer::preview`] and consumed
/// by [`super::init::ProjectInitializer::commit`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitPreview {
    pub project_yaml_path: PathBuf,
    pub specs_dir: PathBuf,
    pub project_yaml_bytes: String,
}

impl ProjectConfig {
    /// Load the `.workmen/` configuration of `root`.
    ///
    /// Returns [`Ok(None)`] when `.workmen/project.yaml` does not exist
    /// (the project is un-initialized). When the file exists, parses it
    /// and every spec file under `.workmen/specs/*.yaml`.
    ///
    /// # Errors
    ///
    /// * [`WorkmenError::Config`] — the YAML is malformed, the schema
    ///   version is unsupported, or the active profile is not present
    ///   in the listed profiles.
    /// * [`WorkmenError::Io`] — a file or directory exists but cannot
    ///   be read.
    pub fn load(root: &ProjectRoot) -> Result<Option<Self>, WorkmenError> {
        let root_path = root.path();
        let dot_workmen = root_path.join(".workmen");
        let project_yaml_path = dot_workmen.join("project.yaml");

        // Project-relative path for error reporting. If root.path() is
        // absolute (it always is), we strip it from the absolute path
        // to produce a relative one.
        let relative = |abs: &Path| -> PathBuf {
            abs.strip_prefix(root_path)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| abs.to_path_buf())
        };

        if !project_yaml_path.exists() {
            return Ok(None);
        }

        let yaml_text = std::fs::read_to_string(&project_yaml_path)
            .map_err(|e| WorkmenError::io(&relative(&project_yaml_path), e))?;
        let project_yaml: ProjectYaml = serde_yaml::from_str(&yaml_text).map_err(|e| {
            WorkmenError::config(
                format!("invalid .workmen/project.yaml: {e}"),
                &relative(&project_yaml_path),
            )
        })?;

        // Validate that the active profile is listed in the profiles
        // vector. An empty list is allowed for un-initialized drafts but
        // for a loaded config the active profile must be referenced.
        if !project_yaml.profiles.is_empty()
            && !project_yaml
                .profiles
                .iter()
                .any(|p| p == &project_yaml.active_profile)
        {
            return Err(WorkmenError::config(
                format!(
                    "activeProfile {:?} is not listed in profiles {:?}",
                    project_yaml.active_profile, project_yaml.profiles
                ),
                &relative(&project_yaml_path),
            ));
        }

        // Load spec files. Missing `specs/` directory is treated as an
        // empty spec set (e.g. a freshly-initialized project). Each
        // spec file must be parseable.
        let specs_dir = dot_workmen.join("specs");
        let mut specs: Vec<SpecFile> = Vec::new();
        if specs_dir.is_dir() {
            let entries = std::fs::read_dir(&specs_dir)
                .map_err(|e| WorkmenError::io(&relative(&specs_dir), e))?;
            // Sort entries by file name for deterministic ordering.
            let mut paths: Vec<PathBuf> = entries
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|p| {
                    p.is_file()
                        && p.extension()
                            .and_then(|s| s.to_str())
                            .is_some_and(|s| s == "yaml" || s == "yml")
                })
                .collect();
            paths.sort();
            for spec_path in paths {
                let text = std::fs::read_to_string(&spec_path)
                    .map_err(|e| WorkmenError::io(&relative(&spec_path), e))?;
                let profile: Profile = serde_yaml::from_str(&text).map_err(|e| {
                    WorkmenError::config(
                        format!("invalid profile spec: {e}"),
                        &relative(&spec_path),
                    )
                })?;
                let project_relative_spec = relative(&spec_path);
                let in_specs = project_relative_spec
                    .strip_prefix(".workmen/specs")
                    .map(Path::to_path_buf)
                    .unwrap_or(project_relative_spec);
                let spec_path_str = in_specs.to_string_lossy().into_owned();
                specs.push(SpecFile {
                    path: spec_path_str,
                    profile,
                });
            }
        }

        Ok(Some(Self {
            project_yaml,
            specs,
        }))
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
        let dir = std::env::temp_dir().join(format!("workmen-cfg-{label}-{pid}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("create tempdir");
        dir
    }

    fn remove_tempdir(path: &Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn round_trip_write_then_load() {
        let tmp = fresh_tempdir("round-trip");
        let dot_workmen = tmp.join(".workmen");
        std::fs::create_dir_all(&dot_workmen).unwrap();

        let yaml = ProjectYaml {
            schema_version: 1,
            id: "demo".into(),
            active_profile: ProfileId("default".into()),
            profiles: vec![ProfileId("default".into())],
        };
        let rendered = serde_yaml::to_string(&yaml).unwrap();
        std::fs::write(dot_workmen.join("project.yaml"), &rendered).unwrap();

        let root = ProjectRoot::discover(&tmp).unwrap();
        let loaded = ProjectConfig::load(&root).unwrap().expect("Some(cfg)");
        assert_eq!(loaded.project_yaml, yaml);
        assert!(loaded.specs.is_empty());

        remove_tempdir(&tmp);
    }

    #[test]
    fn invalid_yaml_returns_config_error_with_path() {
        let tmp = fresh_tempdir("invalid-yaml");
        let dot_workmen = tmp.join(".workmen");
        std::fs::create_dir_all(&dot_workmen).unwrap();
        std::fs::write(
            dot_workmen.join("project.yaml"),
            "schemaVersion: 1\n\tprofiles: [\n",
        )
        .unwrap();

        let root = ProjectRoot::discover(&tmp).unwrap();
        let err = ProjectConfig::load(&root).unwrap_err();
        match err {
            WorkmenError::Config { path, .. } => {
                assert!(!path.is_absolute(), "path must be project-relative");
                assert!(path.ends_with(".workmen/project.yaml"));
            }
            other => panic!("expected Config error, got {other:?}"),
        }

        remove_tempdir(&tmp);
    }
}
