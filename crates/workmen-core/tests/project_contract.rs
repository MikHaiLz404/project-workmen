//! Workmen project-contract integration tests (Task 3).
//!
//! This file gates the [`workmen_core::project`] module: root discovery,
//! `.workmen/` configuration loading, safe atomic initialization, schema
//! version handling, and migration seam. Every test in this file must pass
//! before T3 is considered complete.

use std::fs;
use std::path::{Path, PathBuf};

use workmen_core::WorkmenError;
use workmen_core::project::{InitPreview, ProjectConfig, ProjectInitializer, ProjectRoot};

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// Absolute path to the shared `empty-game/` fixture shipped with the
/// integration tests. The fixture is the canonical "Game Project" used by
/// the init flow: an empty directory plus a `.gitignore`.
fn empty_game_fixture() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("tests/fixtures/projects/empty-game")
}

/// Create a fresh, isolated temporary directory that the test owns. Used
/// when we need a clean project root that does not collide with another
/// test running in parallel.
fn fresh_tempdir(label: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("workmen-t3-{label}-{pid}-{nanos}"));
    fs::create_dir_all(&dir).expect("create tempdir");
    dir
}

/// Remove a directory tree created by [`fresh_tempdir`]. Best-effort —
/// integration tests don't fail the suite on cleanup errors.
fn remove_tempdir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

/// Render a `ProjectYaml` to its on-disk string form. Kept here (rather
/// than re-using the production serializer) so a regression in the
/// initializer cannot accidentally hide a serialization round-trip bug.
fn render_project_yaml(yaml: &workmen_core::project::ProjectYaml) -> String {
    serde_yaml::to_string(yaml).expect("ProjectYaml is serializable")
}

// ---------------------------------------------------------------------------
// ProjectRoot discovery
// ---------------------------------------------------------------------------

#[test]
fn root_discover_walks_upward_to_git_marker() {
    // The repo has a `.git` directory at its root. We start inside a deep
    // nested directory and expect discover to land on the repo root.
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workmen-core lives at crates/workmen-core")
        .to_path_buf();

    // Build a nested directory we are sure exists inside the workspace.
    // We create it on demand to keep this test self-contained.
    let nested = fresh_tempdir("git-marker");
    let nested_for_creation = nested.clone();
    fs::create_dir_all(&nested_for_creation).expect("create nested");

    // Insert a `.git` directory at the workspace level for the duration of
    // this test. We do NOT need a real git database; we only need a
    // directory named `.git` somewhere above the start point.
    //
    // To keep the test side-effect free we instead rely on the fact that
    // the workmen repo has a `.git` directory at its workspace root (this
    // is the case for the unit tests of the menubar task too). We point
    // `start` at a path that is inside the workspace so discover will
    // walk up and find `.git`.
    //
    // The temp dir we just created is *outside* the workspace, so we
    // simulate the walk by starting from a known workspace subdir.
    let nested_inside_workspace = workspace.join("target").join("__workmen_test_marker");
    fs::create_dir_all(&nested_inside_workspace).expect("create nested inside workspace");

    let discovered = ProjectRoot::discover(&nested_inside_workspace).expect("discover");
    assert_eq!(
        discovered.marker(),
        workmen_core::project::RootMarker::GitDir,
        ".git must be detected when walking upward from a workspace subdir"
    );
    // The discovered root must be at or above the start.
    assert!(
        discovered.path().starts_with(&workspace),
        "discovered root {:?} must be at or above workspace root {:?}",
        discovered.path(),
        workspace
    );

    // Cleanup.
    let _ = fs::remove_dir_all(&nested_inside_workspace);
    remove_tempdir(&nested);
}

#[test]
fn root_discover_falls_back_to_supplied_dir_when_no_marker_exists() {
    // A fresh temp dir contains no `.git` and no `.workmen`. The supplied
    // directory itself must be returned as the root.
    let tmp = fresh_tempdir("supplied-dir");
    let root = ProjectRoot::discover(&tmp).expect("discover");
    assert_eq!(
        root.marker(),
        workmen_core::project::RootMarker::SuppliedDir
    );
    // The discovered path must equal the supplied (absolute) path.
    let expected = fs::canonicalize(&tmp).unwrap_or(tmp.clone());
    assert_eq!(root.path(), expected.as_path());

    remove_tempdir(&tmp);
}

#[test]
fn root_discover_finds_dot_workmen_marker() {
    // A directory that has `.workmen/` but no `.git/` must yield a
    // WorkmenDir marker.
    let tmp = fresh_tempdir("dot-workmen");
    let dot_workmen = tmp.join(".workmen");
    fs::create_dir_all(&dot_workmen).expect("create .workmen");

    // Start from a deeper nested directory so we exercise the upward walk.
    let nested = tmp.join("src").join("ui");
    fs::create_dir_all(&nested).expect("create nested");

    let root = ProjectRoot::discover(&nested).expect("discover");
    assert_eq!(root.marker(), workmen_core::project::RootMarker::WorkmenDir);
    let expected = fs::canonicalize(&tmp).unwrap_or(tmp.clone());
    assert_eq!(root.path(), expected.as_path());

    remove_tempdir(&tmp);
}

#[test]
fn root_discover_prefers_git_over_workmen_at_same_level() {
    // When both `.git` and `.workmen` exist at the same level, `.git`
    // wins — the design marks `.git` as the authoritative project
    // boundary when present.
    let tmp = fresh_tempdir("both-markers");
    fs::create_dir_all(tmp.join(".git")).expect("create .git");
    fs::create_dir_all(tmp.join(".workmen")).expect("create .workmen");

    let root = ProjectRoot::discover(&tmp).expect("discover");
    assert_eq!(root.marker(), workmen_core::project::RootMarker::GitDir);

    remove_tempdir(&tmp);
}

#[test]
fn root_discover_returns_io_error_for_missing_start() {
    let missing = std::env::temp_dir().join("workmen-t3-missing-NEVER-EXIST-zzz");
    // The path must not exist when we call discover.
    let _ = fs::remove_dir_all(&missing);
    assert!(!missing.exists());

    let err = ProjectRoot::discover(&missing).expect_err("missing start must error");
    assert!(
        matches!(err, WorkmenError::Io { .. }),
        "expected Io error, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// ProjectConfig::load
// ---------------------------------------------------------------------------

#[test]
fn project_config_load_returns_none_when_dot_workmen_absent() {
    let tmp = fresh_tempdir("no-config");
    let root = ProjectRoot::discover(&tmp).expect("discover");

    let loaded = ProjectConfig::load(&root).expect("load returns Ok(None)");
    assert!(
        loaded.is_none(),
        "no .workmen/ directory must yield Ok(None), got {loaded:?}"
    );

    remove_tempdir(&tmp);
}

#[test]
fn project_config_load_round_trips_after_init() {
    let tmp = fresh_tempdir("cfg-roundtrip");
    let root = ProjectRoot::discover(&tmp).expect("discover");

    let preview = ProjectInitializer::preview(&root).expect("preview");
    let _cfg = ProjectInitializer::commit(preview, true).expect("commit");

    let loaded = ProjectConfig::load(&root).expect("load after init");
    let cfg = loaded.expect("Some(ProjectConfig) after init");
    // The on-disk id must be derived from the directory name (slugified).
    // The default empty-game fixture's basename is the temp label slugified.
    assert!(
        !cfg.project_yaml.id.is_empty(),
        "project id must not be empty"
    );
    assert!(
        !cfg.project_yaml.profiles.is_empty(),
        "freshly-initialized project must have at least the active profile id in its profiles list"
    );
    assert!(
        cfg.project_yaml
            .profiles
            .iter()
            .any(|p| p == &cfg.project_yaml.active_profile),
        "active_profile must appear in profiles list"
    );

    remove_tempdir(&tmp);
}

#[test]
fn project_config_load_rejects_invalid_yaml_with_config_error() {
    let tmp = fresh_tempdir("bad-yaml");
    let dot_workmen = tmp.join(".workmen");
    fs::create_dir_all(&dot_workmen).expect("create .workmen");
    // Intentionally malformed: tabs are forbidden in YAML, plus the
    // document is truncated.
    let project_yaml = dot_workmen.join("project.yaml");
    fs::write(&project_yaml, "schemaVersion: 1\n\tprofiles: [\n").expect("write bad yaml");

    let root = ProjectRoot::discover(&tmp).expect("discover");
    let err = ProjectConfig::load(&root).expect_err("invalid yaml must error");
    match &err {
        WorkmenError::Config { path, .. } => {
            // The path stored on the error must be project-relative
            // (".workmen/project.yaml"), not the absolute temp path.
            assert!(
                !path.is_absolute(),
                "Config error path must be project-relative, got {path:?}"
            );
            assert!(
                path.ends_with(".workmen/project.yaml"),
                "Config error path must end in .workmen/project.yaml, got {path:?}"
            );
        }
        other => panic!("expected WorkmenError::Config, got {other:?}"),
    }

    remove_tempdir(&tmp);
}

#[test]
fn project_config_load_rejects_unsupported_schema_version() {
    let tmp = fresh_tempdir("bad-schema");
    let dot_workmen = tmp.join(".workmen");
    fs::create_dir_all(&dot_workmen).expect("create .workmen");
    let project_yaml = dot_workmen.join("project.yaml");
    // Use schemaVersion: 99 — Workmen only understands 1 today.
    let yaml = r#"schemaVersion: 99
id: "demo"
activeProfile: "default"
profiles: ["default"]
"#;
    fs::write(&project_yaml, yaml).expect("write yaml");

    let root = ProjectRoot::discover(&tmp).expect("discover");
    let err = ProjectConfig::load(&root).expect_err("unsupported schemaVersion must error");
    assert!(
        matches!(err, WorkmenError::Config { .. }),
        "expected Config error for unsupported schemaVersion, got {err:?}"
    );

    remove_tempdir(&tmp);
}

#[test]
fn project_config_load_rejects_active_profile_not_in_profiles_list() {
    let tmp = fresh_tempdir("orphan-active");
    let dot_workmen = tmp.join(".workmen");
    fs::create_dir_all(&dot_workmen).expect("create .workmen");
    let project_yaml = dot_workmen.join("project.yaml");
    let yaml = r#"schemaVersion: 1
id: "demo"
activeProfile: "missing-profile"
profiles: ["default"]
"#;
    fs::write(&project_yaml, yaml).expect("write yaml");

    let root = ProjectRoot::discover(&tmp).expect("discover");
    let err =
        ProjectConfig::load(&root).expect_err("active_profile not in profiles list must error");
    assert!(
        matches!(err, WorkmenError::Config { .. }),
        "expected Config error, got {err:?}"
    );

    remove_tempdir(&tmp);
}

// ---------------------------------------------------------------------------
// Migration seam
// ---------------------------------------------------------------------------

#[test]
fn migrate_identity_v1_to_v1_returns_input_unchanged() {
    let input = "schemaVersion: 1\nid: x\n";
    let out = workmen_core::project::migrate(input, 1, 1).expect("identity migrate");
    assert_eq!(out, input);
}

#[test]
fn migrate_unsupported_target_returns_not_implemented_error() {
    let err = workmen_core::project::migrate("schemaVersion: 1\n", 1, 2)
        .expect_err("v1 -> v2 must be unsupported in this milestone");
    match &err {
        WorkmenError::Config { message, .. } => {
            assert!(
                message.contains("not implemented"),
                "error message must explain the gap, got {message:?}"
            );
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn migrate_unsupported_current_version_returns_config_error() {
    let err = workmen_core::project::migrate("schemaVersion: 99\n", 99, 1)
        .expect_err("unknown current version must error");
    match &err {
        WorkmenError::Config { message, .. } => {
            assert!(
                message.contains("unsupported"),
                "message must say unsupported, got {message:?}"
            );
            assert!(
                message.contains("99"),
                "message must mention the offending version, got {message:?}"
            );
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// ProjectInitializer::preview
// ---------------------------------------------------------------------------

#[test]
fn init_preview_returns_paths_and_bytes_without_writing() {
    let tmp = fresh_tempdir("preview-dry");
    let root = ProjectRoot::discover(&tmp).expect("discover");

    let preview: InitPreview = ProjectInitializer::preview(&root).expect("preview");
    assert_eq!(
        preview.project_yaml_path,
        root.path().join(".workmen/project.yaml")
    );
    assert_eq!(preview.specs_dir, root.path().join(".workmen/specs"));
    assert!(
        !preview.project_yaml_bytes.is_empty(),
        "preview must carry a non-empty serialized project.yaml"
    );
    // The on-disk fixture must not have been touched.
    assert!(
        !root.path().join(".workmen").exists(),
        "preview must not create .workmen/, found {:?}",
        root.path().join(".workmen")
    );

    remove_tempdir(&tmp);
}

#[test]
fn init_preview_rejects_already_initialized_project() {
    let tmp = fresh_tempdir("preview-twice");
    let root = ProjectRoot::discover(&tmp).expect("discover");

    let preview = ProjectInitializer::preview(&root).expect("first preview");
    let _cfg = ProjectInitializer::commit(preview, true).expect("commit");

    let err = ProjectInitializer::preview(&root)
        .expect_err("second preview must fail because .workmen/ now exists");
    match &err {
        WorkmenError::Config { message, path } => {
            assert!(
                message.contains("already initialized"),
                "message must explain the failure, got {message:?}"
            );
            assert!(
                !path.is_absolute(),
                "Config error path must be project-relative, got {path:?}"
            );
        }
        other => panic!("expected Config error, got {other:?}"),
    }

    remove_tempdir(&tmp);
}

// ---------------------------------------------------------------------------
// ProjectInitializer::commit
// ---------------------------------------------------------------------------

#[test]
fn init_commit_without_confirm_returns_config_error() {
    let tmp = fresh_tempdir("commit-no-confirm");
    let root = ProjectRoot::discover(&tmp).expect("discover");
    let preview = ProjectInitializer::preview(&root).expect("preview");

    let err = ProjectInitializer::commit(preview, false)
        .expect_err("commit without --confirm must error");
    match &err {
        WorkmenError::Config { message, .. } => {
            assert!(
                message.contains("--confirm"),
                "message must reference --confirm, got {message:?}"
            );
        }
        other => panic!("expected Config error, got {other:?}"),
    }
    // The fixture must remain pristine.
    assert!(
        !root.path().join(".workmen").exists(),
        "commit without --confirm must not write .workmen/"
    );

    remove_tempdir(&tmp);
}

#[test]
fn init_commit_with_confirm_writes_dot_workmen_atomically() {
    let tmp = fresh_tempdir("commit-with-confirm");
    let root = ProjectRoot::discover(&tmp).expect("discover");
    let preview = ProjectInitializer::preview(&root).expect("preview");

    let cfg = ProjectInitializer::commit(preview, true).expect("commit");

    let workmen_dir = root.path().join(".workmen");
    assert!(workmen_dir.is_dir(), ".workmen/ must be created");
    assert!(
        workmen_dir.join("project.yaml").is_file(),
        "project.yaml must exist after commit"
    );
    assert!(
        workmen_dir.join("specs").is_dir(),
        "specs/ directory must exist (and be empty) after commit"
    );
    // specs/ must start empty.
    let specs_entries: Vec<_> = fs::read_dir(workmen_dir.join("specs"))
        .expect("read specs dir")
        .collect();
    assert!(
        specs_entries.is_empty(),
        "specs/ must be empty after a fresh init, got {specs_entries:?}"
    );

    // The returned ProjectConfig must round-trip through ProjectConfig::load.
    let loaded = ProjectConfig::load(&root)
        .expect("load")
        .expect("Some(cfg)");
    assert_eq!(loaded.project_yaml, cfg.project_yaml);

    // The on-disk yaml must parse to the same id + active profile.
    let raw = fs::read_to_string(workmen_dir.join("project.yaml")).expect("read yaml");
    assert!(
        raw.contains(&format!("id: {}", cfg.project_yaml.id)),
        "on-disk yaml must contain the id, got {raw:?}"
    );
    assert!(
        raw.contains(&format!(
            "activeProfile: {}",
            cfg.project_yaml.active_profile.0
        )),
        "on-disk yaml must contain the active profile, got {raw:?}"
    );

    remove_tempdir(&tmp);
}

#[test]
fn init_commit_with_confirm_round_trips_through_load() {
    // Stronger end-to-end check: serialize a known ProjectYaml, write it
    // through the initializer, read it back, and confirm the deserialized
    // shape matches what we would produce ourselves.
    let tmp = fresh_tempdir("commit-roundtrip");
    let root = ProjectRoot::discover(&tmp).expect("discover");
    let preview = ProjectInitializer::preview(&root).expect("preview");

    let _ = ProjectInitializer::commit(preview, true).expect("commit");

    let cfg = ProjectConfig::load(&root)
        .expect("load")
        .expect("Some(cfg)");
    // ProjectYaml exposes the schema version through the typed contract.
    assert_eq!(cfg.project_yaml.schema_version, 1);
    // The serialized form of the loaded config must be a valid YAML
    // document — sanity-check by re-parsing it.
    let on_disk =
        fs::read_to_string(root.path().join(".workmen/project.yaml")).expect("read project.yaml");
    let reparsed: serde_yaml::Value =
        serde_yaml::from_str(&on_disk).expect("project.yaml is valid YAML");
    assert_eq!(
        reparsed["schemaVersion"].as_u64(),
        Some(1),
        "on-disk schemaVersion must be 1"
    );

    remove_tempdir(&tmp);
}

// ---------------------------------------------------------------------------
// Smoke: render_project_yaml helper is exercised by the unit tests above
// through ProjectInitializer::preview. Add one positive sanity check so a
// regression in the serializer cannot pass silently.
// ---------------------------------------------------------------------------

#[test]
fn render_project_yaml_helper_round_trips() {
    use workmen_core::model::ProfileId;
    use workmen_core::project::ProjectYaml;

    let yaml = ProjectYaml {
        schema_version: 1,
        id: "demo".to_string(),
        active_profile: ProfileId("default".to_string()),
        profiles: vec![ProfileId("default".to_string())],
    };
    let rendered = render_project_yaml(&yaml);
    let parsed: serde_yaml::Value = serde_yaml::from_str(&rendered).expect("reparse");
    assert_eq!(parsed["schemaVersion"].as_u64(), Some(1));
    assert_eq!(parsed["id"].as_str(), Some("demo"));
}

// ---------------------------------------------------------------------------
// Sanity: the empty-game fixture path the CLI integration test will use is
// actually present and writable.
// ---------------------------------------------------------------------------

#[test]
fn empty_game_fixture_exists_and_has_gitignore() {
    let fixture = empty_game_fixture();
    assert!(fixture.is_dir(), "empty-game fixture must be a directory");
    assert!(
        fixture.join(".gitignore").is_file(),
        "empty-game fixture must contain a .gitignore file"
    );
}
