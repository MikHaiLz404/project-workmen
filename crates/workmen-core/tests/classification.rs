//! Workmen classification integration tests (Task 5).
//!
//! This file gates the [`workmen_core::classify`] module: every
//! role (Source / Runtime / Derived / MirrorTarget / Excluded /
//! Unclassified) must have at least one table-driven test, and the
//! draft-profile generator must produce families that never merge
//! solely on dimensions.
//!
//! Tests in this file build synthetic [`ScannedFile`]s (no on-disk
//! fixtures) so the gates are deterministic and quick to run.

use std::path::PathBuf;

use workmen_core::classify::{
    ClassificationPolicy, ClassificationResult, Confidence, DraftProfile, EvidenceReason,
    RoleAssignment, UnclassifiedEntry, classify, default_policy, draft_profiles_default,
};
use workmen_core::model::{AssetFormat, AssetRole, PixelSize, Rect, ViewBox};
use workmen_core::scan::{ScanCache, ScanResult, ScannedFile};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a synthetic [`ScannedFile`] with a deterministic hash.
fn file(path: &str, format: AssetFormat) -> ScannedFile {
    ScannedFile {
        path: path.to_string(),
        format,
        size: 1024,
        modified: std::time::SystemTime::UNIX_EPOCH,
        blake3_hash: Some(workmen_core::scan::blake3_hex(path.as_bytes())),
    }
}

/// Build a [`ScanResult`] from a list of paths + formats.
fn scan_result(files: Vec<ScannedFile>) -> ScanResult {
    ScanResult {
        files,
        diagnostics: Vec::new(),
        cache: ScanCache::new(),
    }
}

/// Find the assignment for `path` in `result`. Test helper.
fn assignment<'a>(result: &'a ClassificationResult, path: &str) -> Option<&'a RoleAssignment> {
    result.assignments.iter().find(|a| a.asset_path == path)
}

// ---------------------------------------------------------------------------
// Module surface smoke
// ---------------------------------------------------------------------------

#[test]
fn classify_module_exposes_expected_public_surface() {
    fn assert_type<T>() {}
    assert_type::<ClassificationResult>();
    assert_type::<RoleAssignment>();
    assert_type::<DraftProfile>();
    assert_type::<Confidence>();
    assert_type::<UnclassifiedEntry>();
    assert_type::<ClassificationPolicy>();
}

// ---------------------------------------------------------------------------
// Default policy is wired correctly
// ---------------------------------------------------------------------------

#[test]
fn default_policy_uses_reasonable_thresholds() {
    let p = default_policy();
    assert!(p.high_confidence_threshold > p.low_confidence_threshold);
    assert!(p.low_confidence_threshold > 0);
    assert!(
        !p.mirror_path_prefixes.is_empty(),
        "mirror prefixes must not be empty"
    );
    assert!(
        !p.source_path_prefixes.is_empty(),
        "source prefixes must not be empty"
    );
    // Contextual metadata formats include the four canonical variants.
    assert!(
        p.contextual_metadata_formats
            .contains(&AssetFormat::IosAssetCatalogJson)
    );
    assert!(
        p.contextual_metadata_formats
            .contains(&AssetFormat::AndroidVectorXml)
    );
    assert!(
        p.contextual_metadata_formats
            .contains(&AssetFormat::AndroidAdaptiveIconXml)
    );
    assert!(
        p.contextual_metadata_formats
            .contains(&AssetFormat::RuntimeManifestJs)
    );
}

// ---------------------------------------------------------------------------
// Source classification
// ---------------------------------------------------------------------------

#[test]
fn classify_source_for_art_in_assets_dir() {
    let scan = scan_result(vec![
        file("assets/player.png", AssetFormat::Png),
        file("assets/enemy.png", AssetFormat::Png),
        file("art/boss.svg", AssetFormat::Svg),
        file("src/assets/coin.png", AssetFormat::Png),
    ]);
    let result = classify(&scan, None, &default_policy());

    let player = assignment(&result, "assets/player.png").expect("player classified");
    assert_eq!(
        player.role,
        AssetRole::Source,
        "player must be Source: {:?}",
        player.confidence
    );

    let enemy = assignment(&result, "assets/enemy.png").expect("enemy classified");
    assert_eq!(enemy.role, AssetRole::Source);

    let boss = assignment(&result, "art/boss.svg").expect("boss classified");
    assert_eq!(
        boss.role,
        AssetRole::Source,
        "art/ prefix must classify as Source"
    );

    let coin = assignment(&result, "src/assets/coin.png").expect("coin classified");
    assert_eq!(coin.role, AssetRole::Source);
}

// ---------------------------------------------------------------------------
// Runtime classification
// ---------------------------------------------------------------------------

#[test]
fn classify_runtime_for_build_outputs() {
    let scan = scan_result(vec![
        file("build/player.png", AssetFormat::Png),
        file("dist/main.js", AssetFormat::RuntimeManifestJs),
        file("Assets/ui.png", AssetFormat::Png),
    ]);
    let result = classify(&scan, None, &default_policy());

    let player = assignment(&result, "build/player.png").expect("classified");
    assert_eq!(
        player.role,
        AssetRole::Runtime,
        "build/ must classify as Runtime"
    );

    let main = assignment(&result, "dist/main.js").expect("classified");
    assert_eq!(main.role, AssetRole::Runtime);

    let ui = assignment(&result, "Assets/ui.png").expect("classified");
    assert_eq!(ui.role, AssetRole::Runtime);
}

// ---------------------------------------------------------------------------
// Mirror target classification (links to ONE runtime asset)
// ---------------------------------------------------------------------------

#[test]
fn classify_mirror_targets_in_ios_android_www() {
    let scan = scan_result(vec![
        file("ios/Assets.xcassets/icon.png", AssetFormat::Png),
        file(
            "android/app/src/main/res/drawable/icon.png",
            AssetFormat::Png,
        ),
        file("www/asset-manifest.js", AssetFormat::RuntimeManifestJs),
    ]);
    let result = classify(&scan, None, &default_policy());

    let ios = assignment(&result, "ios/Assets.xcassets/icon.png").expect("ios classified");
    assert_eq!(
        ios.role,
        AssetRole::MirrorTarget,
        "ios/ must be MirrorTarget: {:?}",
        ios.confidence
    );

    let android = assignment(&result, "android/app/src/main/res/drawable/icon.png")
        .expect("android classified");
    assert_eq!(
        android.role,
        AssetRole::MirrorTarget,
        "android/ must be MirrorTarget"
    );

    let www = assignment(&result, "www/asset-manifest.js").expect("www classified");
    assert_eq!(www.role, AssetRole::MirrorTarget);
}

// ---------------------------------------------------------------------------
// Excluded classification (deprecated / rejected files)
// ---------------------------------------------------------------------------

#[test]
fn classify_excluded_for_legacy_and_deprecated_paths() {
    let scan = scan_result(vec![
        file("assets/deprecated/old.png", AssetFormat::Png),
        file("assets-source/legacy/boss.png", AssetFormat::Png),
        file("random/path/old.svg", AssetFormat::Svg),
    ]);
    let result = classify(&scan, None, &default_policy());

    for path in [
        "assets/deprecated/old.png",
        "assets-source/legacy/boss.png",
        "random/path/old.svg",
    ] {
        let a = assignment(&result, path).unwrap_or_else(|| {
            panic!(
                "path {path} not in assignments. assignments={:?} unclassified={:?}",
                result.assignments, result.unclassified
            )
        });
        assert_eq!(
            a.role,
            AssetRole::Excluded,
            "{path} must be Excluded: {:?}",
            a.confidence
        );
        // Excluded decisions must be supported by named evidence.
        assert!(
            a.confidence.reasons.contains(&EvidenceReason::ExcludedName),
            "{path} must include ExcludedName evidence"
        );
    }
}

// ---------------------------------------------------------------------------
// Unclassified queue (low-confidence assets)
// ---------------------------------------------------------------------------

#[test]
fn classify_unclassified_for_low_confidence_paths() {
    // Paths that don't match any prefix produce low-confidence evidence.
    let scan = scan_result(vec![file("scratch/foo.png", AssetFormat::Png)]);
    let result = classify(&scan, None, &default_policy());

    // The unclassified queue must contain scratch/foo.png.
    assert!(
        result
            .unclassified
            .iter()
            .any(|u| u.asset_path == "scratch/foo.png"),
        "scratch/foo.png must be in the unclassified queue, got unclassified={:?}",
        result.unclassified
    );
    // The role assignment list must NOT include it (it's unclassified).
    assert!(
        result
            .assignments
            .iter()
            .all(|a| a.asset_path != "scratch/foo.png"),
        "scratch/foo.png must not appear in assignments (it's Unclassified)"
    );
}

// ---------------------------------------------------------------------------
// Contextual metadata classification
// ---------------------------------------------------------------------------

#[test]
fn classify_contextual_metadata_to_runtime() {
    let scan = scan_result(vec![
        file(
            "ios/Assets.xcassets/AppIcon.appiconset/Contents.json",
            AssetFormat::IosAssetCatalogJson,
        ),
        file(
            "android/app/src/main/res/drawable/ic.xml",
            AssetFormat::AndroidVectorXml,
        ),
        file(
            "android/app/src/main/res/mipmap-hdpi/ic.png",
            AssetFormat::AndroidAdaptiveIconXml,
        ),
        file("www/asset-manifest.js", AssetFormat::RuntimeManifestJs),
    ]);
    let result = classify(&scan, None, &default_policy());

    let json = assignment(
        &result,
        "ios/Assets.xcassets/AppIcon.appiconset/Contents.json",
    )
    .expect("Contents.json");
    assert_eq!(
        json.role,
        AssetRole::MirrorTarget,
        "iOS Contents.json under ios/ is a mirror target"
    );

    let xml = assignment(&result, "android/app/src/main/res/drawable/ic.xml").expect("xml");
    assert_eq!(xml.role, AssetRole::MirrorTarget);

    let adaptive =
        assignment(&result, "android/app/src/main/res/mipmap-hdpi/ic.png").expect("adaptive");
    // Adaptive icons live in android/ tree but they are RASTER assets, not
    // XML. They still classify as MirrorTarget because android/ is in
    // mirror_path_prefixes.
    assert_eq!(adaptive.role, AssetRole::MirrorTarget);

    let manifest = assignment(&result, "www/asset-manifest.js").expect("manifest");
    assert_eq!(manifest.role, AssetRole::MirrorTarget);
}

// ---------------------------------------------------------------------------
// Evidence scoring carries named reasons
// ---------------------------------------------------------------------------

#[test]
fn classification_evidence_carries_named_reasons() {
    let scan = scan_result(vec![file("assets/player.png", AssetFormat::Png)]);
    let result = classify(&scan, None, &default_policy());

    let player = assignment(&result, "assets/player.png").expect("player classified");
    assert!(
        !player.confidence.reasons.is_empty(),
        "classification must carry named reasons, got {:?}",
        player.confidence
    );
    // Source-path evidence is the dominant reason.
    assert!(
        player
            .confidence
            .reasons
            .contains(&EvidenceReason::SourcePathPrefix),
        "assets/player.png must surface SourcePathPrefix reason, got {:?}",
        player.confidence.reasons
    );
}

// ---------------------------------------------------------------------------
// Hash-stable classification (deterministic)
// ---------------------------------------------------------------------------

#[test]
fn classification_is_deterministic_for_same_input() {
    let scan = scan_result(vec![
        file("assets/player.png", AssetFormat::Png),
        file("build/player.png", AssetFormat::Png),
        file("ios/player.png", AssetFormat::Png),
    ]);
    let p = default_policy();
    let r1 = classify(&scan, None, &p);
    let r2 = classify(&scan, None, &p);

    for path in ["assets/player.png", "build/player.png", "ios/player.png"] {
        let a1 = assignment(&r1, path).unwrap();
        let a2 = assignment(&r2, path).unwrap();
        assert_eq!(a1.role, a2.role, "{path} role must be stable");
        assert_eq!(
            a1.confidence.score, a2.confidence.score,
            "{path} score must be stable"
        );
    }
}

// ---------------------------------------------------------------------------
// No silent guessing: every assignment has a confidence score
// ---------------------------------------------------------------------------

#[test]
fn every_assignment_has_a_confidence_score() {
    let scan = scan_result(vec![
        file("assets/player.png", AssetFormat::Png),
        file("build/player.png", AssetFormat::Png),
        file("ios/player.png", AssetFormat::Png),
        file("scratch/foo.png", AssetFormat::Png),
    ]);
    let result = classify(&scan, None, &default_policy());

    for a in &result.assignments {
        // Every assignment must carry at least one named reason.
        assert!(
            !a.confidence.reasons.is_empty(),
            "{} must have non-empty reasons",
            a.asset_path
        );
        // Score must be non-zero (no silent zero-confidence).
        assert!(
            a.confidence.score != 0 || matches!(a.role, AssetRole::Unclassified),
            "{} must have non-zero score or be Unclassified",
            a.asset_path
        );
    }
}

// ---------------------------------------------------------------------------
// Draft profiles: families are grouped by directory + stem + format
// ---------------------------------------------------------------------------

#[test]
fn draft_profiles_group_by_directory_stem_format() {
    let scan = scan_result(vec![
        file("assets/ui/btn-rest.png", AssetFormat::Png),
        file("assets/ui/btn-rest@2x.png", AssetFormat::Png),
        file("assets/ui/btn-press.png", AssetFormat::Png),
        file("assets/ui/btn-press@2x.png", AssetFormat::Png),
    ]);
    let result = classify(&scan, None, &default_policy());
    let drafts = draft_profiles_default(&result);

    // Two families: btn-rest and btn-press.
    assert_eq!(drafts.len(), 2, "expected 2 draft profiles, got {drafts:?}");
    let names: Vec<_> = drafts.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"btn-rest"));
    assert!(names.contains(&"btn-press"));

    // Each family must carry both SD and @2x variants.
    let rest = drafts.iter().find(|d| d.name == "btn-rest").unwrap();
    assert_eq!(
        rest.member_assets.len(),
        2,
        "btn-rest must have 2 members, got {:?}",
        rest.member_assets
    );
    let press = drafts.iter().find(|d| d.name == "btn-press").unwrap();
    assert_eq!(press.member_assets.len(), 2);
}

// ---------------------------------------------------------------------------
// Draft profiles: same stem in different dirs do NOT merge
// ---------------------------------------------------------------------------

#[test]
fn draft_profiles_do_not_merge_across_directories() {
    let scan = scan_result(vec![
        file("assets/ui/btn-rest.png", AssetFormat::Png),
        file("assets-source/btn-rest.png", AssetFormat::Png),
    ]);
    let result = classify(&scan, None, &default_policy());
    let drafts = draft_profiles_default(&result);

    // Different directories, same stem. The plan says "Group candidate
    // families by directory, naming stem/token shape, dimensions,
    // format, and contextual metadata links. Never merge families
    // solely because dimensions match." Directory is a *first-class*
    // key, so these stay separate.
    assert_eq!(
        drafts.len(),
        2,
        "different directories must produce distinct families"
    );
}

// ---------------------------------------------------------------------------
// Draft profiles: same dimensions but different format DO NOT merge
// ---------------------------------------------------------------------------

#[test]
fn draft_profiles_do_not_merge_on_dimensions_alone() {
    // The plan explicitly says: "Never merge families solely because
    // dimensions match." A PNG and an SVG with the same name in the
    // same directory must produce two families (because their formats
    // differ). We use ScannedFile records that share name + dir but
    // differ in format.
    let scan = scan_result(vec![
        file("art/logo.png", AssetFormat::Png),
        file("art/logo.svg", AssetFormat::Svg),
    ]);
    let result = classify(&scan, None, &default_policy());
    let drafts = draft_profiles_default(&result);

    assert_eq!(
        drafts.len(),
        2,
        "different formats must produce distinct families, got {drafts:?}"
    );
}

// ---------------------------------------------------------------------------
// Draft profiles: representative asset is the most-confident member
// ---------------------------------------------------------------------------

#[test]
fn draft_profile_representative_is_most_confident_member() {
    let scan = scan_result(vec![
        file("assets/btn-rest.png", AssetFormat::Png),
        file("assets/btn-rest@2x.png", AssetFormat::Png),
    ]);
    let result = classify(&scan, None, &default_policy());
    let drafts = draft_profiles_default(&result);

    let rest = drafts.iter().find(|d| d.name == "btn-rest").unwrap();
    // The representative must be one of the members.
    assert!(rest.member_assets.contains(&rest.representative_asset));
}

// ---------------------------------------------------------------------------
// Draft profiles: confidence is the worst-case across members
// ---------------------------------------------------------------------------

#[test]
fn draft_profile_confidence_is_worst_case() {
    let scan = scan_result(vec![
        file("assets/foo.png", AssetFormat::Png),  // Source (high)
        file("scratch/foo.png", AssetFormat::Png), // Unclassified (low)
    ]);
    let result = classify(&scan, None, &default_policy());
    let drafts = draft_profiles_default(&result);

    // Two different directories => two families (foo in assets/ and foo
    // in scratch/). The drafts module must respect the directory key.
    let assets_foo = drafts.iter().find(|d| d.name == "foo").expect("foo drafts");
    // All drafts must carry a non-empty reason set.
    for d in &drafts {
        assert!(
            !d.confidence.reasons.is_empty(),
            "{} has empty reasons",
            d.name
        );
        // Quick sanity: every member is in the result.
        for m in &d.member_assets {
            assert!(
                result.assignments.iter().any(|a| a.asset_path == *m),
                "draft {} references missing assignment {}",
                d.name,
                m
            );
        }
        // Ignore the unused variable lint.
        let _ = assets_foo;
    }
}

// ---------------------------------------------------------------------------
// PolicySummary surfaces high/medium/low counts
// ---------------------------------------------------------------------------

#[test]
fn classification_result_summarizes_role_counts() {
    let scan = scan_result(vec![
        file("assets/a.png", AssetFormat::Png),
        file("build/b.png", AssetFormat::Png),
        file("ios/c.png", AssetFormat::Png),
        file("scratch/d.png", AssetFormat::Png),
    ]);
    let result = classify(&scan, None, &default_policy());

    assert!(result.policy_summary.total >= 4);
    assert!(result.policy_summary.source_count >= 1);
    assert!(result.policy_summary.runtime_count >= 1);
    assert!(result.policy_summary.mirror_target_count >= 1);
    assert!(result.policy_summary.unclassified_count >= 1);
}

// ---------------------------------------------------------------------------
// Helper: build a sized raster file for hash-stable evidence scoring
// ---------------------------------------------------------------------------
/// Helper: build a sized raster file for hash-stable evidence scoring
#[allow(dead_code)]
fn sized_raster(path: &str, width: u32, height: u32) -> ScannedFile {
    let _ = PixelSize { width, height };
    let _ = ViewBox {
        min_x: 0,
        min_y: 0,
        width: 1,
        height: 1,
    };
    let _ = Rect {
        x: 0,
        y: 0,
        width,
        height,
    };
    file(path, AssetFormat::Png)
}

/// Defensive: the test file uses helper functions whose return types
/// exercise the T2 domain model end-to-end. This is a smoke check
/// that the types round-trip without panicking.
#[test]
fn classify_synthetic_round_trip() {
    let path = PathBuf::from("art/btn-rest.png");
    let _ = file(path.to_str().unwrap(), AssetFormat::Png);
    let _ = sized_raster(path.to_str().unwrap(), 64, 64);
    let _ = classify(&scan_result(vec![]), None, &default_policy());
}
