//! Text renderer for Workmen reports.

use super::Report;

pub fn render(report: &Report) -> String {
    match report {
        Report::Scan(scan) => render_scan(scan),
        Report::Validation(validation) => render_validation(validation),
    }
}

fn render_scan(scan: &super::ScanReport) -> String {
    let mut out = String::new();
    out.push_str("Workmen scan\n");
    out.push_str(&format!("  root: {}\n", scan.root.display()));
    out.push_str(&format!("  files: {} file(s)\n", scan.files.len()));
    out.push_str(&format!(
        "  diagnostics: {} diagnostic(s)\n",
        scan.diagnostics.len()
    ));
    if !scan.diagnostics.is_empty() {
        out.push('\n');
        out.push_str("  diagnostics:\n");
        for d in &scan.diagnostics {
            out.push_str(&format!("    [{:?}] {} -- {}\n", d.kind, d.path, d.message));
        }
    }
    if !scan.files.is_empty() {
        out.push('\n');
        out.push_str("  files:\n");
        for f in scan.files.iter().take(20) {
            let hash = f.blake3_hash.as_deref().map(|h| &h[..8]).unwrap_or("-");
            out.push_str(&format!(
                "    {} ({:?}, {} bytes, blake3:{})\n",
                f.path, f.format, f.size, hash
            ));
        }
        if scan.files.len() > 20 {
            out.push_str(&format!("    ... and {} more\n", scan.files.len() - 20));
        }
    }
    out
}

fn render_validation(validation: &super::ValidationReport) -> String {
    let mut out = String::new();
    out.push_str("Workmen validation\n");
    out.push_str(&format!("  root: {}\n", validation.root.display()));
    out.push_str(&format!(
        "  profile: {} on {:?}\n",
        validation.profile_id.0, validation.platform
    ));
    out.push_str(&format!("  issues: {} issue(s)\n", validation.issues.len()));
    if !validation.issues.is_empty() {
        out.push('\n');
        out.push_str("  issues:\n");
        for issue in &validation.issues {
            let path = &issue.asset_path;
            let diff = &issue.diff;
            out.push_str(&format!(
                "    [{}] {} -- rule={} expected={} actual={} -- {}\n",
                format!("{:?}", diff.severity).to_lowercase(),
                path,
                diff.rule_id,
                diff.expected,
                diff.actual,
                diff.suggested_action,
            ));
        }
    }
    out
}
