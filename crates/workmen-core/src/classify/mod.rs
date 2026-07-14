//! Asset classification.
//!
//! `classify` assigns an [`AssetRole`] to each file in a
//! [`ScanResult`] using *evidence scoring* with named reasons. The
//! policy is one typed `ClassificationPolicy` (no magic numbers).
//!
//! `draft_profiles` groups the classified assets into candidate
//! families and proposes draft [`Profile`]s. Draft profiles are
//! *not* written during scan; they are returned to the caller for
//! review.
//!
//! The classifier never silently guesses. Low-confidence assets
//! enter the [`UnclassifiedEntry`] queue so the user can resolve
//! them.

mod draft;
mod families;
mod roles;

pub use draft::{DraftProfile, UnresolvedConflict, draft_profiles, draft_profiles_default};
pub use families::{FamilyGroup, FamilyKey, group_assignments_with_format, group_into_families};
pub use roles::{
    ClassificationPolicy, ClassificationResult, Confidence, Evidence, EvidenceReason,
    PolicySummary, RoleAssignment, UnclassifiedEntry, classify,
};

/// Default classification policy. Conservative: high threshold to
/// avoid guessing; mirror-target bias so `ios/` / `www/` / `android/`
/// assets surface as `MirrorTarget` (per design §12).
pub fn default_policy() -> ClassificationPolicy {
    ClassificationPolicy {
        high_confidence_threshold: 60,
        low_confidence_threshold: 20,
        runtime_path_prefixes: vec![
            "build/".to_string(),
            "dist/".to_string(),
            "runtime/".to_string(),
            "Assets/".to_string(),
        ],
        mirror_path_prefixes: vec![
            "ios/".to_string(),
            "android/".to_string(),
            "www/".to_string(),
            "public/".to_string(),
        ],
        excluded_names: vec![
            "deprecated".to_string(),
            "old".to_string(),
            "legacy".to_string(),
        ],
        excluded_extensions: vec![".tmp".to_string(), ".bak".to_string()],
        contextual_metadata_formats: [
            crate::model::AssetFormat::IosAssetCatalogJson,
            crate::model::AssetFormat::AndroidVectorXml,
            crate::model::AssetFormat::AndroidAdaptiveIconXml,
            crate::model::AssetFormat::RuntimeManifestJs,
        ]
        .into_iter()
        .collect(),
        source_path_prefixes: vec![
            "assets/".to_string(),
            "art/".to_string(),
            "src/assets/".to_string(),
            "source/".to_string(),
            "assets-source/".to_string(),
        ],
    }
}
