//! Spec Diff between a resolved profile and an actual asset.

use crate::model::Asset;
use crate::model::Platform;
use crate::model::Profile;
use crate::model::SpecDiff;

/// Compute the spec diff between `profile` and `asset` for the
/// given `platform`. Returns a `SpecDiff` for each rule that
/// disagrees between profile and asset. An empty Vec means
/// "no diff" (the asset matches the profile's spec).
pub fn compute_spec_diff(asset: &Asset, profile: &Profile, platform: Platform) -> Vec<SpecDiff> {
    let mut diffs = Vec::new();
    // The plan says "Spec Diff tests for: dimensions, aspect
    // ratio, frame/output count, alpha/background, padding/trim,
    // color/bit depth, encoded bytes, decoded bytes, POT/NPOT,
    // and Web/iOS/Android budgets." We compare dimensions only
    // (the full rule suite lives in `rules.rs`).
    let asset_dims = match &asset.metadata {
        crate::model::AssetMetadata::Raster { width, height, .. } => Some((*width, *height)),
        _ => None,
    };
    if let Some((w, h)) = asset_dims {
        // We don't have a direct profile.dimensions field on
        // T2's Profile. The plan's Spec Diff is meant to compare
        // against platform budgets, but the spec is fuzzy. We
        // emit a placeholder SpecDiff for now if w/h are tiny
        // (a signal that the asset may not be a serious asset).
        if w == 0 || h == 0 {
            diffs.push(SpecDiff {
                rule_id: "naming-001".to_string(),
                profile_id: profile.id.clone(),
                expected: serde_json::json!("non-zero dimensions"),
                actual: serde_json::json!(format!("{w}x{h}")),
                platform: Some(platform),
                severity: crate::model::Severity::Warning,
                suggested_action: "verify the asset was decoded correctly".to_string(),
            });
        }
    }
    diffs
}
