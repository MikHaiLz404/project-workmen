//! CLI integration tests for the `workmen init` subcommand.
//!
//! These tests exercise the real binary against a temporary copy of
//! the empty-game fixture. The fixture is *copied* (not symlinked) to
//! a fresh tempdir for each test so the upward walk in
//! `ProjectRoot::discover` does not pick up a `.git` marker from
//! outside the test boundary (which would cause `discover` to
//! return the parent of the test tempdir as the project root).
//!
//! The empty-game source fixture lives at
//! `crates/workmen-core/tests/fixtures/projects/empty-game/`; each
//! CLI test copies it to a fresh tempdir via
//! [`copy_fixture_to_tempdir`].
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;

/// Path to the empty-game source fixture under
/// `crates/workmen-core/tests/fixtures/`.
fn empty_game_source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("workmen-core")
        .join("tests")
        .join("fixtures")
        .join("projects")
        .join("empty-game")
}

/// Copy the empty-game source fixture into a fresh tempdir. Returns
/// the tempdir path. The tempdir is `tempdir/workmen-init-{label}-{pid}-{nanos}`
/// so concurrent test runs cannot collide.
fn copy_fixture_to_tempdir(label: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("workmen-init-{label}-{pid}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create tempdir");
    copy_dir_contents(&empty_game_source(), &dir).expect("copy fixture");
    dir
}

/// Recursive directory copy. Used to clone the empty-game fixture
/// into a fresh tempdir.
fn copy_dir_contents(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_contents(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[test]
fn init_preview_prints_paths_and_exits_2_without_confirm() {
    let fixture = copy_fixture_to_tempdir("preview");

    Command::cargo_bin("workmen")
        .unwrap()
        .arg("init")
        .arg(&fixture)
        .assert()
        .failure()
        .code(2)
        .stderr(predicates::str::contains("init preview for"))
        .stderr(predicates::str::contains("will create"))
        .stderr(predicates::str::contains("--confirm"));

    let _ = std::fs::remove_dir_all(&fixture);
}

#[test]
fn init_commit_with_confirm_creates_dot_workmen() {
    let fixture = copy_fixture_to_tempdir("with-confirm");

    Command::cargo_bin("workmen")
        .unwrap()
        .arg("init")
        .arg(&fixture)
        .arg("--confirm")
        .assert()
        .success();

    let dot_workmen = fixture.join(".workmen");
    assert!(
        dot_workmen.is_dir(),
        ".workmen/ should exist after --confirm"
    );
    assert!(
        dot_workmen.join("project.yaml").is_file(),
        ".workmen/project.yaml should exist after --confirm"
    );
    assert!(
        dot_workmen.join("specs").is_dir(),
        ".workmen/specs/ should exist after --confirm"
    );
    let specs_entries: Vec<_> = std::fs::read_dir(dot_workmen.join("specs"))
        .expect("read specs/")
        .collect();
    assert!(
        specs_entries.is_empty(),
        ".workmen/specs/ should start empty, got {specs_entries:?}"
    );

    let _ = std::fs::remove_dir_all(&fixture);
}

#[test]
fn init_without_confirm_does_not_modify_project() {
    let fixture = copy_fixture_to_tempdir("no-confirm");

    Command::cargo_bin("workmen")
        .unwrap()
        .arg("init")
        .arg(&fixture)
        .assert()
        .failure();

    assert!(
        !fixture.join(".workmen").exists(),
        ".workmen/ must not be created when --confirm is omitted"
    );

    let _ = std::fs::remove_dir_all(&fixture);
}
