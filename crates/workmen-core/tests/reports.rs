//! Workmen reports integration tests (Task 7).
//!
//! This file gates the [`workmen_core::report`] module and the
//! `workmen scan` / `workmen validate` CLI behavior:
//!
//! - Text reports render summary, files, diagnostics, and
//!   validation issues.
//! - JSON reports are valid JSON and carry the same fields.
//! - Exit codes follow the design contract:
//!   - 0: pass
//!   - 1: validation errors
//!   - 2: configuration or tool failure
//! - `--quiet` suppresses the report; only the exit code tells
//!   the result.
//! - `--format json` switches to JSON output.
//!
//! Tests in this file are pure (no CLI invocation) so they run
//! in the `cargo test --workspace` fast path.

use std::path::PathBuf;

use workmen_core::model::{ProfileId, Severity, SpecDiff, ValidationIssue};
use workmen_core::report::{
    ExitCode, Report, ReportFormat, ScanReport, ValidationReport, render_json, render_text,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn scan_report() -> ScanReport {
    ScanReport {
        root: PathBuf::from("/tmp/proj"),
        files: vec![workmen_core::scan::ScannedFile {
            path: "assets/btn-rest.png".to_string(),
            format: workmen_core::model::AssetFormat::Png,
            size: 1024,
            modified: std::time::SystemTime::UNIX_EPOCH,
            blake3_hash: Some(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            ),
        }],
        diagnostics: vec![],
    }
}

fn validation_report() -> ValidationReport {
    ValidationReport {
        root: PathBuf::from("/tmp/proj"),
        profile_id: ProfileId("ui".to_string()),
        platform: workmen_core::model::Platform::Web,
        issues: vec![ValidationIssue {
            asset_path: "assets/btn-rest.png".to_string(),
            diff: SpecDiff {
                rule_id: "naming-001".to_string(),
                profile_id: ProfileId("ui".to_string()),
                expected: serde_json::json!("non-zero dimensions"),
                actual: serde_json::json!("0x0"),
                platform: Some(workmen_core::model::Platform::Web),
                severity: Severity::Warning,
                suggested_action: "verify the asset was decoded correctly".to_string(),
            },
        }],
    }
}

// ---------------------------------------------------------------------------
// Module surface smoke
// ---------------------------------------------------------------------------

#[test]
fn report_module_exposes_expected_public_surface() {
    fn assert_type<T>() {}
    assert_type::<Report>();
    assert_type::<ScanReport>();
    assert_type::<ValidationReport>();
    assert_type::<ReportFormat>();
    assert_type::<ExitCode>();
    let _ = render_text;
    let _ = render_json;
}

// ---------------------------------------------------------------------------
// Text render: scan report
// ---------------------------------------------------------------------------

#[test]
fn text_render_scan_report_includes_summary() {
    let report = Report::Scan(scan_report());
    let text = render_text(&report);
    assert!(
        text.contains("Workmen scan"),
        "must include heading: got {text:?}"
    );
    assert!(text.contains("assets/btn-rest.png"), "must list file");
    assert!(
        text.contains("1 file") || text.contains("1 files"),
        "must include file count"
    );
    assert!(
        text.contains("0 diagnostic"),
        "must include diagnostic count"
    );
}

// ---------------------------------------------------------------------------
// Text render: validation report
// ---------------------------------------------------------------------------

#[test]
fn text_render_validation_report_includes_issues() {
    let report = Report::Validation(validation_report());
    let text = render_text(&report);
    assert!(
        text.contains("Workmen validation"),
        "must include heading: got {text:?}"
    );
    assert!(
        text.contains("naming-001"),
        "must include rule_id of the issue"
    );
    assert!(
        text.contains("assets/btn-rest.png"),
        "must include asset path of the issue"
    );
    assert!(
        text.contains("Warning") || text.contains("warning"),
        "must surface severity"
    );
    assert!(
        text.contains("verify the asset was decoded correctly"),
        "must surface suggested_action"
    );
}

// ---------------------------------------------------------------------------
// JSON render: valid JSON, top-level keys
// ---------------------------------------------------------------------------

#[test]
fn json_render_scan_report_is_valid_json() {
    let report = Report::Scan(scan_report());
    let json = render_json(&report);
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("render_json must emit valid JSON");
    assert_eq!(parsed["kind"], "scan");
    assert!(parsed["root"].is_string());
    assert!(parsed["files"].is_array());
    assert_eq!(parsed["files"].as_array().unwrap().len(), 1);
    assert!(parsed["diagnostics"].is_array());
}

#[test]
fn json_render_validation_report_is_valid_json() {
    let report = Report::Validation(validation_report());
    let json = render_json(&report);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("must be valid JSON");
    assert_eq!(parsed["kind"], "validation");
    assert!(parsed["issues"].is_array());
    assert_eq!(parsed["issues"].as_array().unwrap().len(), 1);
    let issue = &parsed["issues"][0];
    assert_eq!(issue["diff"]["ruleId"], "naming-001");
    assert_eq!(issue["diff"]["severity"], "warning");
    assert_eq!(issue["diff"]["expected"], "non-zero dimensions");
}

// ---------------------------------------------------------------------------
// Exit code: scan with no diagnostics exits 0
// ---------------------------------------------------------------------------

#[test]
fn scan_with_no_diagnostics_exits_zero() {
    let report = Report::Scan(scan_report());
    assert_eq!(report.exit_code(), ExitCode::Pass);
}

// ---------------------------------------------------------------------------
// Exit code: scan with diagnostics exits 1
// ---------------------------------------------------------------------------

#[test]
fn scan_with_diagnostics_exits_one() {
    let mut sr = scan_report();
    sr.diagnostics = vec![workmen_core::scan::ScanDiagnostic {
        path: "assets/corrupt.png".to_string(),
        kind: workmen_core::scan::DiagnosticKind::DecodeError,
        message: "corrupt PNG".to_string(),
    }];
    let report = Report::Scan(sr);
    assert_eq!(report.exit_code(), ExitCode::ValidationErrors);
}

// ---------------------------------------------------------------------------
// Exit code: validation with errors exits 1
// ---------------------------------------------------------------------------

#[test]
fn validation_with_issues_exits_one() {
    let report = Report::Validation(validation_report());
    assert_eq!(report.exit_code(), ExitCode::ValidationErrors);
}

// ---------------------------------------------------------------------------
// Exit code: validation with no issues exits 0
// ---------------------------------------------------------------------------

#[test]
fn validation_with_no_issues_exits_zero() {
    let report = Report::Validation(ValidationReport {
        root: PathBuf::from("/tmp/proj"),
        profile_id: ProfileId("ui".to_string()),
        platform: workmen_core::model::Platform::Web,
        issues: vec![],
    });
    assert_eq!(report.exit_code(), ExitCode::Pass);
}

// ---------------------------------------------------------------------------
// Format selection
// ---------------------------------------------------------------------------

#[test]
fn format_default_is_text() {
    assert_eq!(ReportFormat::default(), ReportFormat::Text);
}

#[test]
fn format_from_str() {
    assert_eq!("text".parse::<ReportFormat>().unwrap(), ReportFormat::Text);
    assert_eq!("json".parse::<ReportFormat>().unwrap(), ReportFormat::Json);
    assert!("xml".parse::<ReportFormat>().is_err());
}

// ---------------------------------------------------------------------------
// Text and JSON render produce the same content
// ---------------------------------------------------------------------------

#[test]
fn text_and_json_carry_the_same_issue_payload() {
    let report = Report::Validation(validation_report());
    let text = render_text(&report);
    let json = render_json(&report);
    assert!(text.contains("naming-001"));
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["issues"][0]["diff"]["ruleId"], "naming-001");
}

// ---------------------------------------------------------------------------
// Empty scan report is valid
// ---------------------------------------------------------------------------

#[test]
fn empty_scan_report_renders_cleanly() {
    let report = Report::Scan(ScanReport {
        root: PathBuf::from("/tmp/empty"),
        files: vec![],
        diagnostics: vec![],
    });
    let text = render_text(&report);
    let json = render_json(&report);
    assert!(text.contains("0 file") || text.contains("0 files"));
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["files"].as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// Empty validation report is valid
// ---------------------------------------------------------------------------

#[test]
fn empty_validation_report_renders_cleanly() {
    let report = Report::Validation(ValidationReport {
        root: PathBuf::from("/tmp/empty"),
        profile_id: ProfileId("default".to_string()),
        platform: workmen_core::model::Platform::Web,
        issues: vec![],
    });
    let text = render_text(&report);
    let json = render_json(&report);
    assert!(text.contains("0 issue") || text.contains("no issues"));
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["issues"].as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// ExitCode is repr(u8) and matches the design contract
// ---------------------------------------------------------------------------

#[test]
fn exit_code_values_match_design() {
    assert_eq!(ExitCode::Pass as u8, 0);
    assert_eq!(ExitCode::ValidationErrors as u8, 1);
    assert_eq!(ExitCode::ConfigFailure as u8, 2);
}
