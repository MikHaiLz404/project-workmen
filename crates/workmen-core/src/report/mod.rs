//! Reports, logs, and CLI exit contracts.
//!
//! The design's §7 (Spec Diff and Validation Console) defines the
//! CLI exit codes:
//!
//! - `0`: validation passed
//! - `1`: validation errors
//! - `2`: configuration or tool failure
//!
//! This module exposes:
//! - [`Report`] — the union of a [`ScanReport`] and a
//!   [`ValidationReport`]. The CLI renders one of these to
//!   either text or JSON.
//! - [`ReportFormat`] — text or json, selected by `--format`.
//! - [`ExitCode`] — the design's 0/1/2 contract.
//! - [`render_text`] / [`render_json`] — pure renderers.
//!
//! Logging is configured separately via `tracing` (the design's
//! §8). This module does not initialize the tracing subscriber;
//! that lives in the CLI binary so tests do not pull in
//! log-formatter dependencies.

mod json;
mod text;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::Platform;
use crate::model::ProfileId;
use crate::model::ValidationIssue;
use crate::scan::ScanDiagnostic;
use crate::scan::ScannedFile;

/// The design's exit-code contract.
///
/// `repr(u8)` so the CLI can `std::process::exit(code as u8)` and
/// tests can assert the exact numeric value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ExitCode {
    /// `0`: validation passed.
    Pass = 0,
    /// `1`: validation errors.
    ValidationErrors = 1,
    /// `2`: configuration or tool failure.
    ConfigFailure = 2,
}

/// Output format, selected by `--format`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    #[default]
    Text,
    Json,
}

impl std::fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportFormat::Text => f.write_str("text"),
            ReportFormat::Json => f.write_str("json"),
        }
    }
}

impl std::str::FromStr for ReportFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!("unknown report format: {other}")),
        }
    }
}

/// A scanner result, surfaced as a report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScanReport {
    pub root: PathBuf,
    pub files: Vec<ScannedFile>,
    pub diagnostics: Vec<ScanDiagnostic>,
}

/// A validation result, surfaced as a report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub root: PathBuf,
    pub profile_id: ProfileId,
    pub platform: Platform,
    pub issues: Vec<ValidationIssue>,
}

/// The union of every report Workmen produces. Each CLI command
/// builds one variant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Report {
    Scan(ScanReport),
    Validation(ValidationReport),
}

impl Report {
    /// Compute the CLI exit code for this report. Mirrors the
    /// design's §7 contract.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Report::Scan(scan) => {
                if scan.diagnostics.is_empty() {
                    ExitCode::Pass
                } else {
                    ExitCode::ValidationErrors
                }
            }
            Report::Validation(validation) => {
                if validation.issues.is_empty() {
                    ExitCode::Pass
                } else {
                    ExitCode::ValidationErrors
                }
            }
        }
    }

    /// Render the report in the chosen format.
    pub fn render(&self, format: ReportFormat) -> String {
        match format {
            ReportFormat::Text => text::render(self),
            ReportFormat::Json => json::render(self),
        }
    }
}

/// Render `report` as text. Convenience wrapper that lets the
/// CLI call `render_text(&report)` without importing the
/// `text` submodule.
pub fn render_text(report: &Report) -> String {
    text::render(report)
}

/// Render `report` as JSON.
pub fn render_json(report: &Report) -> String {
    json::render(report)
}
