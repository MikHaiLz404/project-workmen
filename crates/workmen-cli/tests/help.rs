use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn help_names_the_read_only_scan_command() {
    Command::cargo_bin("workmen")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("scan"))
        .stdout(contains("validate"))
        .stdout(contains("init"));
}

#[test]
fn version_prints_binary_name_and_version() {
    Command::cargo_bin("workmen")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("workmen"))
        .stdout(contains("0.1.0"));
}
