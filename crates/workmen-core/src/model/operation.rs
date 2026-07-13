// SPDX-License-Identifier: MIT OR Apache-2.0
//! Structured operation log events.
//!
//! Operation events are append-only JSONL records the Workmen runtime
//! emits as it works. They describe **what Workmen did** (scanned a
//! directory, classified an asset, exported a profile, ...), not **what
//! is wrong** with the project — that is the validation report's job.
//! Keeping the two streams separate lets the CLI flag one without
//! shipping the other.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The kind of action an [`OperationEvent`] describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OperationKind {
    /// A project scan.
    Scan,
    /// A classification pass.
    Classify,
    /// A validation pass.
    Validate,
    /// A derived-export pass.
    Export,
    /// A packing step (atlas / sprite sheet generation).
    Pack,
    /// A mirror-copy step for a target platform.
    Mirror,
    /// Generic structured log line. Prefer the specific kinds whenever
    /// they apply.
    Log,
}

/// One structured operation log event.
///
/// `timestamp` is an ISO-8601 string (`chrono` is intentionally avoided
/// to keep the dependency surface small — future revisions will replace
/// `String` with `chrono::DateTime<Utc>`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    pub timestamp: String,
    pub kind: OperationKind,
    #[serde(rename = "assetPath")]
    pub asset_path: Option<String>,
    pub message: String,
    #[serde(rename = "inputHash")]
    pub input_hash: Option<String>,
    #[serde(rename = "outputHash")]
    pub output_hash: Option<String>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn operation_kind_strings_match_design() {
        assert_eq!(
            serde_json::to_value(OperationKind::Scan).unwrap(),
            json!("scan")
        );
        assert_eq!(
            serde_json::to_value(OperationKind::Classify).unwrap(),
            json!("classify")
        );
        assert_eq!(
            serde_json::to_value(OperationKind::Validate).unwrap(),
            json!("validate")
        );
        assert_eq!(
            serde_json::to_value(OperationKind::Export).unwrap(),
            json!("export")
        );
        assert_eq!(
            serde_json::to_value(OperationKind::Pack).unwrap(),
            json!("pack")
        );
        assert_eq!(
            serde_json::to_value(OperationKind::Mirror).unwrap(),
            json!("mirror")
        );
        assert_eq!(
            serde_json::to_value(OperationKind::Log).unwrap(),
            json!("log")
        );
    }

    #[test]
    fn operation_event_round_trips() {
        let event = OperationEvent {
            timestamp: "2026-07-13T00:00:00Z".into(),
            kind: OperationKind::Export,
            asset_path: Some("assets/coin.png".into()),
            message: "wrote web preview".into(),
            input_hash: Some("blake3:abc".into()),
            output_hash: None,
            duration_ms: Some(42),
        };
        let v = serde_json::to_value(&event).unwrap();
        assert_eq!(v["kind"], json!("export"));
        assert_eq!(v["assetPath"], json!("assets/coin.png"));
        assert_eq!(v["durationMs"], json!(42));
        let back: OperationEvent = serde_json::from_value(v).unwrap();
        assert_eq!(back, event);
    }
}
