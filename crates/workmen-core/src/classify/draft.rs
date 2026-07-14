//! Draft-profile generation.
//!
//! `draft_profiles` consumes a [`ClassificationResult`] and emits
//! one [`DraftProfile`] per candidate family. Draft profiles are
//! *not* persisted by this module; the caller decides when to
//! write them. The plan says: "Produce Draft Profiles containing
//! proposed matchers, observed ranges, representative assets,
//! confidence, and unresolved conflicts. Do not write them
//! during scan."

use std::collections::BTreeMap;

use crate::model::AssetFormat;
use crate::model::Profile;
use crate::model::ProfileId;

use super::families::{FamilyGroup, group_assignments_with_format};
use super::roles::{ClassificationResult, Confidence};

/// One draft profile produced by [`draft_profiles`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftProfile {
    /// The proposed profile id (e.g. `"btn-rest"` for a family of
    /// `btn-rest`/`btn-rest@2x`).
    pub id: ProfileId,
    /// Human-readable name (matches the stem of the family).
    pub name: String,
    /// Observed asset format. PNG vs SVG matters: a draft
    /// profile for a PNG family must not match SVG assets.
    pub observed_format: AssetFormat,
    /// The most-confident member asset, picked by walking the
    /// assignments in the family.
    pub representative_asset: String,
    /// All assets in this family.
    pub member_assets: Vec<String>,
    /// Confidence is the *worst-case* (minimum) score across the
    /// family's members. A high-confidence family needs every
    /// member to be high-confidence; one low-confidence member
    /// drags the whole family down.
    pub confidence: Confidence,
    /// Reasons collected from the family's assignments (deduped).
    pub member_roles: Vec<String>,
    /// Optional Profile to seed; `None` if this is a fresh draft.
    pub seed: Option<Profile>,
}

/// A conflict inside a family. The plan says: "Produce Draft
/// Profiles containing … unresolved conflicts." A conflict is a
/// member whose role does not match the family's dominant role
/// (e.g. one Excluded file in an otherwise-Source family).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedConflict {
    pub asset_path: String,
    pub reason: String,
}

/// Build draft profiles from a [`ClassificationResult`].
///
/// Format is taken from a side-table the caller passes in (the
/// `format_for` closure). The classifier currently does not embed
/// format in `RoleAssignment`, so the caller must look it up by
/// scanning the original scan result or by reading the asset's
/// path extension.
pub fn draft_profiles<F>(result: &ClassificationResult, format_for: F) -> Vec<DraftProfile>
where
    F: Fn(&str) -> AssetFormat,
{
    let groups = group_assignments_with_format(&result.assignments, &format_for);
    groups.into_iter().map(draft_from_group).collect()
}

/// Convenience overload that derives format from path extension.
/// PNG/JPG/WebP/SVG match by file extension; everything else is
/// `AssetFormat::Other`. The plan does not yet require the
/// classifier to embed format in `RoleAssignment`; this helper
/// keeps the caller boilerplate-free until that lands.
pub fn draft_profiles_default(result: &ClassificationResult) -> Vec<DraftProfile> {
    draft_profiles(result, |path| {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        match ext.as_deref() {
            Some("png") => AssetFormat::Png,
            Some("jpg") | Some("jpeg") => AssetFormat::Jpg,
            Some("webp") => AssetFormat::WebP,
            Some("svg") => AssetFormat::Svg,
            _ => AssetFormat::Other(path.to_string()),
        }
    })
}

fn draft_from_group(group: FamilyGroup) -> DraftProfile {
    let FamilyGroup {
        key,
        member_assignments,
    } = group;

    let member_assets: Vec<String> = member_assignments
        .iter()
        .map(|a| a.asset_path.clone())
        .collect();

    // Confidence is the worst-case across members.
    let min_confidence = member_assignments
        .iter()
        .map(|a| a.confidence.score)
        .min()
        .unwrap_or(0);
    // Collect all distinct reasons.
    let mut all_reasons = Vec::new();
    for a in &member_assignments {
        for r in &a.confidence.reasons {
            if !all_reasons.contains(r) {
                all_reasons.push(*r);
            }
        }
    }
    let confidence = Confidence {
        score: min_confidence,
        reasons: all_reasons,
    };

    // Representative: the asset with the highest confidence score.
    let representative = member_assignments
        .iter()
        .max_by_key(|a| a.confidence.score)
        .map(|a| a.asset_path.clone())
        .unwrap_or_default();

    // Member roles summary: collect the distinct role labels.
    let mut role_counts: BTreeMap<String, usize> = BTreeMap::new();
    for a in &member_assignments {
        *role_counts.entry(format!("{:?}", a.role)).or_insert(0) += 1;
    }
    let member_roles: Vec<String> = role_counts.into_keys().collect();

    DraftProfile {
        id: ProfileId(key.stem.clone()),
        name: key.stem,
        observed_format: key.format,
        representative_asset: representative,
        member_assets,
        confidence,
        member_roles,
        seed: None,
    }
}
