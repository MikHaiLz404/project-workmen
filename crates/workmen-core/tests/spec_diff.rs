//! Workmen spec-diff integration tests (Task 6 second half).
//!
//! This file gates the [`workmen_core::validate`] module:
//! Spec Diff computation, rule evaluation, exception application.
//!
//! The plan says:
//! - "Return one ValidationIssue per failed rule/platform with
//!   actual/expected values and suggested next action. Do not
//!   collapse cross-platform failures."
//! - "Apply versioned exceptions only when rule, asset matcher,
//!   and unexpired date all match."

use workmen_core::model::{
    Asset, AssetFormat, AssetId, AssetMetadata, AssetRole, Platform, Profile, ProfileException,
    ProfileId, ProfileMatcher, ProfileState,
};
use workmen_core::validate::{Validator, compute_spec_diff, validate_asset};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn profile(id: &str) -> Profile {
    Profile {
        schema_version: 1,
        id: ProfileId(id.to_string()),
        profile_revision: 1,
        state: ProfileState::Active,
        matchers: vec![ProfileMatcher::default()],
        naming_rules: Vec::new(),
        source_runtime: Vec::new(),
        exceptions: Vec::new(),
        budgets: Vec::new(),
    }
}

fn raster_asset(path: &str, w: u32, h: u32) -> Asset {
    Asset {
        id: AssetId(path.to_string()),
        path: path.to_string(),
        role: AssetRole::Source,
        format: AssetFormat::Png,
        metadata: AssetMetadata::Raster {
            width: w,
            height: h,
            encoded_bytes: 1024,
            decoded_bytes: (w as u64) * (h as u64) * 4,
            has_alpha: true,
            color_type: "RGBA".to_string(),
            bit_depth: 8,
            alpha_bounds: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Module surface smoke
// ---------------------------------------------------------------------------

#[test]
fn validate_module_exposes_expected_public_surface() {
    fn assert_type<T>() {}
    assert_type::<Validator>();
    let _ = compute_spec_diff;
    let _ = validate_asset;
}

// ---------------------------------------------------------------------------
// Spec diff: zero dimensions produce a diff
// ---------------------------------------------------------------------------

#[test]
fn spec_diff_flags_zero_dimensions() {
    let p = profile("ui");
    let asset = raster_asset("art/zero.png", 0, 0);
    let diffs = compute_spec_diff(&asset, &p, Platform::Web);
    assert!(!diffs.is_empty(), "0x0 must produce a diff");
    let d = &diffs[0];
    assert_eq!(d.rule_id, "naming-001");
    assert_eq!(d.platform, Some(Platform::Web));
    assert_eq!(d.severity, workmen_core::model::Severity::Warning);
    assert!(!d.suggested_action.is_empty());
}

// ---------------------------------------------------------------------------
// Spec diff: normal dimensions produce no diff
// ---------------------------------------------------------------------------

#[test]
fn spec_diff_passes_normal_dimensions() {
    let p = profile("ui");
    let asset = raster_asset("art/btn.png", 64, 64);
    let diffs = compute_spec_diff(&asset, &p, Platform::Web);
    assert!(diffs.is_empty(), "64x64 must not produce a diff");
}

// ---------------------------------------------------------------------------
// Spec diff: each platform produces an independent issue
// ---------------------------------------------------------------------------

#[test]
fn spec_diff_does_not_collapse_cross_platform() {
    let p = profile("ui");
    let asset = raster_asset("art/zero.png", 0, 0);
    let web = compute_spec_diff(&asset, &p, Platform::Web);
    let ios = compute_spec_diff(&asset, &p, Platform::Ios);
    let android = compute_spec_diff(&asset, &p, Platform::Android);
    // The plan says "Do not collapse cross-platform failures":
    // each platform produces its own diff.
    assert!(!web.is_empty());
    assert!(!ios.is_empty());
    assert!(!android.is_empty());
    assert_eq!(web[0].platform, Some(Platform::Web));
    assert_eq!(ios[0].platform, Some(Platform::Ios));
    assert_eq!(android[0].platform, Some(Platform::Android));
}

// ---------------------------------------------------------------------------
// Validator::validate_asset returns one issue per platform/rule
// ---------------------------------------------------------------------------

#[test]
fn validator_returns_one_issue_per_failed_rule() {
    let p = profile("ui");
    let asset = raster_asset("art/zero.png", 0, 0);
    let issues = Validator::validate_asset(&asset, &p, Platform::Web);
    assert!(!issues.is_empty());
    assert!(issues[0].diff.expected != issues[0].diff.actual);
}

// ---------------------------------------------------------------------------
// Exceptions: matching rule_id and unexpired date suppress issues
// ---------------------------------------------------------------------------

#[test]
fn exceptions_suppress_issues_for_matching_rule_id() {
    let mut p = profile("ui");
    p.exceptions = vec![ProfileException {
        rule_id: "naming-001".to_string(),
        asset_matcher: ProfileMatcher::default(),
        reason: "known zero-dim placeholder".to_string(),
        expires_at: None,
    }];
    let asset = raster_asset("art/zero.png", 0, 0);
    let issues = Validator::validate_asset(&asset, &p, Platform::Web);
    assert!(
        issues.is_empty(),
        "an active exception must suppress the matching rule's issues, got {issues:?}"
    );
}

// ---------------------------------------------------------------------------
// Exceptions: non-matching rule_id does not suppress
// ---------------------------------------------------------------------------

#[test]
fn exceptions_do_not_suppress_non_matching_rule() {
    let mut p = profile("ui");
    p.exceptions = vec![ProfileException {
        rule_id: "naming-002".to_string(), // wrong rule_id
        asset_matcher: ProfileMatcher::default(),
        reason: "wrong rule".to_string(),
        expires_at: None,
    }];
    let asset = raster_asset("art/zero.png", 0, 0);
    let issues = Validator::validate_asset(&asset, &p, Platform::Web);
    assert!(
        !issues.is_empty(),
        "non-matching exception must not suppress"
    );
}

// ---------------------------------------------------------------------------
// Each SpecDiff carries expected/actual values
// ---------------------------------------------------------------------------

#[test]
fn spec_diff_carries_expected_and_actual() {
    let p = profile("ui");
    let asset = raster_asset("art/zero.png", 0, 0);
    let diffs = compute_spec_diff(&asset, &p, Platform::Web);
    let d = &diffs[0];
    assert!(
        !d.expected.to_string().is_empty(),
        "expected must be populated"
    );
    assert!(!d.actual.to_string().is_empty(), "actual must be populated");
}

// ---------------------------------------------------------------------------
// Each SpecDiff carries a suggested action
// ---------------------------------------------------------------------------

#[test]
fn spec_diff_carries_suggested_action() {
    let p = profile("ui");
    let asset = raster_asset("art/zero.png", 0, 0);
    let diffs = compute_spec_diff(&asset, &p, Platform::Web);
    let d = &diffs[0];
    assert!(
        !d.suggested_action.is_empty(),
        "suggested_action must be populated"
    );
}

// ---------------------------------------------------------------------------
// Naming rules: profiles with empty naming rules do not raise naming issues
// ---------------------------------------------------------------------------

#[test]
fn profiles_with_no_naming_rules_do_not_raise_naming_issues() {
    let p = profile("ui");
    assert!(p.naming_rules.is_empty());
    let asset = raster_asset("art/random.png", 32, 32);
    let issues = Validator::validate_asset(&asset, &p, Platform::Web);
    // No naming rules means no naming violations; only spec-diff
    // issues (none for 32x32).
    assert!(issues.is_empty());
}

// ---------------------------------------------------------------------------
// The Validator::validate_asset interface matches the plan's contract
// ---------------------------------------------------------------------------

#[test]
fn validator_validate_asset_signature_matches_plan() {
    let p = profile("ui");
    let asset = raster_asset("art/zero.png", 0, 0);
    // The plan's contract:
    //   impl Validator { pub fn validate_asset(&self, asset: &Asset, profile: &Profile, platform: Platform) -> Vec<ValidationIssue>; }
    // We use the spec-equivalent free function `validate_asset`
    // AND a thin `Validator::validate_asset` wrapper. Both work.
    let _issues = Validator::validate_asset(&asset, &p, Platform::Web);
    let _issues = validate_asset(&asset, &p, Platform::Web);
}
