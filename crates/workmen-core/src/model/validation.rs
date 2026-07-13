// SPDX-License-Identifier: MIT OR Apache-2.0
//! Validation domain model.
//!
//! Outputs of the Workmen validator are described by three nested types:
//!
//! * [`SpecDiff`] — a per-rule, per-platform expected/actual comparison.
//! * [`ValidationIssue`] — a [`SpecDiff`] pinned to the asset that
//!   triggered it.
//! * [`Severity`] — the level of the issue (error / warning / info).
//!
//! Validation reports are separate from [`crate::model::operation::OperationEvent`]s:
//! validation reports tell the user what is wrong; operation events tell
//! them what Workmen did. Keeping the two vocabularies distinct lets the
//! Validation Console filter by severity without dragging in log lines,
//! and lets the operation log stream without re-parsing validation
//! payloads.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::profile::{Platform, ProfileId};

/// Severity of a validation issue. Distinct from operation log levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// The expected-vs-actual comparison the validator emits.
///
/// `expected` and `actual` are `serde_json::Value` so a single diff type
/// can carry scalar numbers, strings, booleans, or compound values
/// (color-space strings, dimension objects, ...). Downstream renderers
/// format them based on the [`SpecDiff::rule_id`] namespace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpecDiff {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    #[serde(rename = "profileId")]
    pub profile_id: ProfileId,
    pub expected: Value,
    pub actual: Value,
    /// Optional platform scope. `None` means the rule applies across all
    /// platforms or is inherently platform-independent.
    pub platform: Option<Platform>,
    pub severity: Severity,
    /// A short, human-readable action the user can take to bring the
    /// asset back in spec.
    #[serde(rename = "suggestedAction")]
    pub suggested_action: String,
}

/// One validation issue pinned to the asset that triggered it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    #[serde(rename = "assetPath")]
    pub asset_path: String,
    pub diff: SpecDiff,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn severity_lowercase_strings() {
        assert_eq!(
            serde_json::to_value(Severity::Error).unwrap(),
            json!("error")
        );
        assert_eq!(
            serde_json::to_value(Severity::Warning).unwrap(),
            json!("warning")
        );
        assert_eq!(serde_json::to_value(Severity::Info).unwrap(), json!("info"));
    }

    #[test]
    fn spec_diff_round_trip_preserves_values() {
        let diff = SpecDiff {
            rule_id: "texture.maxWidth".into(),
            profile_id: ProfileId("web".into()),
            expected: json!(2048),
            actual: json!(4096),
            platform: Some(Platform::Web),
            severity: Severity::Error,
            suggested_action: "downscale".into(),
        };
        let v = serde_json::to_value(&diff).unwrap();
        assert_eq!(v["platform"], json!("web"));
        assert_eq!(v["severity"], json!("error"));
        let back: SpecDiff = serde_json::from_value(v).unwrap();
        assert_eq!(back, diff);
    }
}
