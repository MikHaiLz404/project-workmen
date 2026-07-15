//! JSON renderer for Workmen reports.
//!
//! The plan calls for text, JSON, and SARIF outputs. SARIF is
//! a future task; the JSON renderer here is a stable on-disk
//! format that downstream tools (the desktop workbench, the
//! CI gate) can consume.

use super::Report;

pub fn render(report: &Report) -> String {
    // The Report enum is `#[serde(tag = "kind", rename_all = "lowercase")]`,
    // so `serde_json::to_string_pretty` produces a stable
    // {kind: scan|validation, ...} document.
    serde_json::to_string_pretty(report)
        .unwrap_or_else(|e| format!("{{\"error\": \"failed to serialize report: {e}\"}}"))
}
