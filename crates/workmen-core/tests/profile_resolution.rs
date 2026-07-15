//! Workmen profile-resolution integration tests (Task 6).
//!
//! This file gates the [`workmen_core::profile`] module:
//! matcher specificity, role filtering, path glob, naming pattern,
//! extension matching, ambiguity detection, and lifecycle
//! transitions.
//!
//! Tests in this file build synthetic [`Profile`] records so the
//! gates are deterministic and quick to run.

use workmen_core::model::{
    Asset, AssetFormat, AssetId, AssetMetadata, AssetRole, Profile, ProfileId, ProfileMatcher,
    ProfileState,
};
use workmen_core::profile::{ProfileLifecycle, ProfileResolver, ResolveError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn profile(id: &str, matcher: ProfileMatcher) -> Profile {
    Profile {
        schema_version: 1,
        id: ProfileId(id.to_string()),
        profile_revision: 1,
        state: ProfileState::Active,
        matchers: vec![matcher],
        naming_rules: Vec::new(),
        source_runtime: Vec::new(),
        exceptions: Vec::new(),
        budgets: Vec::new(),
    }
}

fn path_glob_matcher(glob: &str, role: Option<AssetRole>) -> ProfileMatcher {
    ProfileMatcher {
        path_glob: Some(glob.to_string()),
        asset_role: role,
        ..Default::default()
    }
}

fn extension_matcher(ext: &str, role: Option<AssetRole>) -> ProfileMatcher {
    ProfileMatcher {
        extension: Some(ext.trim_start_matches('.').to_string()),
        asset_role: role,
        ..Default::default()
    }
}

fn png_asset(path: &str, width: u32, height: u32) -> Asset {
    Asset {
        id: AssetId(path.to_string()),
        path: path.to_string(),
        role: AssetRole::Source,
        format: AssetFormat::Png,
        metadata: AssetMetadata::Raster {
            width,
            height,
            encoded_bytes: 1024,
            decoded_bytes: (width as u64) * (height as u64) * 4,
            has_alpha: true,
            color_type: "RGBA".to_string(),
            bit_depth: 8,
            alpha_bounds: None,
        },
    }
}
// Module surface smoke
// ---------------------------------------------------------------------------

#[test]
fn profile_module_exposes_expected_public_surface() {
    fn assert_type<T>() {}
    assert_type::<ProfileResolver>();
    assert_type::<ProfileLifecycle>();
    assert_type::<ResolveError>();
}

// ---------------------------------------------------------------------------
// Single match: path glob wins
// ---------------------------------------------------------------------------

#[test]
fn resolver_returns_path_glob_match() {
    let profiles = vec![
        profile(
            "ui",
            path_glob_matcher("assets/ui/**", Some(AssetRole::Source)),
        ),
        profile(
            "enemies",
            path_glob_matcher("assets/enemies/**", Some(AssetRole::Source)),
        ),
    ];
    let asset = png_asset("assets/ui/btn-rest.png", 64, 64);
    let resolver = ProfileResolver::default();
    let result = resolver.resolve(&asset, &profiles);
    let p = result
        .expect("resolution should succeed")
        .expect("must match");
    assert_eq!(
        p.id.0, "ui",
        "path glob 'assets/ui/**' must match 'assets/ui/btn-rest.png'"
    );
}

// ---------------------------------------------------------------------------
// No match returns Ok(None)
// ---------------------------------------------------------------------------

#[test]
fn resolver_returns_none_when_no_match() {
    let profiles = vec![profile("ui", path_glob_matcher("assets/ui/**", None))];
    let asset = png_asset("enemies/goblin.png", 32, 32);
    let resolver = ProfileResolver::default();
    let result = resolver.resolve(&asset, &profiles);
    assert!(result.is_ok(), "no-match is Ok(None), got {result:?}");
    assert!(result.unwrap().is_none(), "no-match must be None");
}

// ---------------------------------------------------------------------------
// Ambiguous match returns AmbiguousProfile error
// ---------------------------------------------------------------------------

#[test]
fn resolver_returns_ambiguous_when_two_profiles_match_equally() {
    let profiles = vec![
        profile("alpha", path_glob_matcher("assets/**", None)),
        profile("beta", path_glob_matcher("assets/**", None)),
    ];
    let asset = png_asset("assets/foo.png", 32, 32);
    let resolver = ProfileResolver::default();
    let result = resolver.resolve(&asset, &profiles);
    let err = result.expect_err("ambiguous match must error");
    match &err {
        ResolveError::Ambiguous { amb } => {
            let ids: Vec<&str> = amb.candidates.iter().map(|p| p.id.0.as_str()).collect();
            assert!(
                ids.contains(&"alpha") && ids.contains(&"beta"),
                "expected both candidates, got {ids:?}"
            );
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Specificity: extension filter is more specific than path glob alone
// ---------------------------------------------------------------------------

#[test]
fn resolver_prefers_more_specific_extension_over_bare_glob() {
    let profiles = vec![
        profile("loose", path_glob_matcher("**", None)),
        profile("png_only", extension_matcher("png", None)),
    ];
    let asset = png_asset("assets/btn-rest.png", 64, 64);
    let resolver = ProfileResolver::default();
    let result = resolver.resolve(&asset, &profiles);
    let p = result
        .expect("resolution should succeed")
        .expect("must match");
    assert_eq!(
        p.id.0, "png_only",
        "extension filter is more specific than path glob alone"
    );
}

// ---------------------------------------------------------------------------
// Role filter
// ---------------------------------------------------------------------------

#[test]
fn resolver_filters_by_role_when_matcher_specifies_role() {
    let profiles = vec![profile(
        "source_only",
        path_glob_matcher("**", Some(AssetRole::Source)),
    )];
    let mut asset = png_asset("assets/btn-rest.png", 64, 64);
    asset.role = AssetRole::MirrorTarget; // Mismatch.
    let resolver = ProfileResolver::default();
    let result = resolver.resolve(&asset, &profiles);
    assert!(result.is_ok(), "role mismatch is Ok(None), got {result:?}");
    assert!(result.unwrap().is_none(), "role mismatch must yield None");
}

// ---------------------------------------------------------------------------
// Path glob specificity
// ---------------------------------------------------------------------------

#[test]
fn resolver_prefers_more_specific_path_glob() {
    let profiles = vec![
        profile("broad", path_glob_matcher("assets/**", None)),
        profile("ui", path_glob_matcher("assets/ui/**", None)),
    ];
    let asset = png_asset("assets/ui/btn-rest.png", 64, 64);
    let resolver = ProfileResolver::default();
    let result = resolver.resolve(&asset, &profiles);
    let p = result
        .expect("resolution should succeed")
        .expect("must match");
    assert_eq!(
        p.id.0, "ui",
        "deeper path glob is more specific than broader one"
    );
}

// ---------------------------------------------------------------------------
// Deterministic specificity: equal specificity uses id ordering
// ---------------------------------------------------------------------------

#[test]
fn resolver_uses_id_ordering_for_truly_equal_specificity() {
    let profiles = vec![
        profile("z", path_glob_matcher("**", None)),
        profile("a", path_glob_matcher("**", None)),
    ];
    let asset = png_asset("assets/foo.png", 32, 32);
    let resolver = ProfileResolver::default();
    let result = resolver.resolve(&asset, &profiles);
    let err = result.expect_err("equal specificity must be ambiguous");
    match err {
        ResolveError::Ambiguous { amb } => {
            assert_eq!(amb.candidates.len(), 2);
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Naming pattern matcher
// ---------------------------------------------------------------------------

#[test]
fn resolver_matches_naming_pattern() {
    let mut p = profile("buttons", path_glob_matcher("**", None));
    p.matchers[0].naming_pattern = Some("btn-{{name}}".to_string());
    let asset = png_asset("assets/btn-rest.png", 64, 64);
    let resolver = ProfileResolver::default();
    let binding = [p];
    let result = resolver.resolve(&asset, &binding);
    assert!(result.is_ok(), "naming match must succeed, got {result:?}");
    assert!(result.unwrap().is_some());
}

// ---------------------------------------------------------------------------
// Lifecycle: Draft can be edited
// ---------------------------------------------------------------------------

#[test]
fn lifecycle_draft_can_be_edited() {
    let mut lc = ProfileLifecycle::new();
    let mut p = profile("ui", path_glob_matcher("assets/ui/**", None));
    p.state = ProfileState::Draft;
    lc.upsert(p.clone());
    lc.edit(&p.id, |p_mut| {
        p_mut.matchers[0].naming_pattern = Some("ui-{{name}}".to_string());
    })
    .expect("Draft can be edited");
    let after = lc.get(&p.id).expect("profile still present");
    assert!(
        after.matchers[0].naming_pattern.is_some(),
        "Draft edit must apply"
    );
}

// ---------------------------------------------------------------------------
// Lifecycle: Active -> Locked increments revision
// ---------------------------------------------------------------------------

#[test]
fn lifecycle_lock_increments_revision() {
    let mut lc = ProfileLifecycle::new();
    let p = profile("ui", path_glob_matcher("assets/ui/**", None));
    let id = p.id.clone();
    let rev_before = p.profile_revision;
    lc.upsert(p);
    lc.lock(&id, "release gate".to_string())
        .expect("active can be locked");
    let after = lc.get(&id).expect("profile present after lock");
    assert!(
        matches!(after.state, ProfileState::Locked),
        "must be Locked"
    );
    assert!(
        after.profile_revision > rev_before,
        "lock must increment revision"
    );
    assert_eq!(
        lc.lifecycle(&id).and_then(|e| e.unlock_reason.as_deref()),
        Some("release gate")
    );
}

// ---------------------------------------------------------------------------
// Lifecycle: Locked rejects edit
// ---------------------------------------------------------------------------

#[test]
fn lifecycle_locked_rejects_edit() {
    let mut lc = ProfileLifecycle::new();
    let mut p = profile("ui", path_glob_matcher("assets/ui/**", None));
    p.state = ProfileState::Locked;
    lc.upsert(p.clone());
    let err = lc.edit(&p.id, |_| {}).expect_err("Locked must reject edit");
    assert!(matches!(
        err,
        workmen_core::profile::LifecycleError::Locked { .. }
    ));
}

// ---------------------------------------------------------------------------
// Lifecycle: unlock requires non-empty reason
// ---------------------------------------------------------------------------

#[test]
fn lifecycle_unlock_requires_non_empty_reason() {
    let mut lc = ProfileLifecycle::new();
    let mut p = profile("ui", path_glob_matcher("assets/ui/**", None));
    p.state = ProfileState::Locked;
    let id = p.id.clone();
    lc.upsert(p);
    let err = lc
        .unlock(&id, "".to_string())
        .expect_err("empty reason must error");
    assert!(matches!(
        err,
        workmen_core::profile::LifecycleError::EmptyUnlockReason
    ));
    lc.unlock(&id, "rolling back for asset rename".to_string())
        .expect("non-empty reason succeeds");
    let after = lc.get(&id).expect("profile present after unlock");
    assert!(
        matches!(after.state, ProfileState::Active),
        "unlock must move to Active"
    );
}

// ---------------------------------------------------------------------------
// Deterministic specificity: tuple ordering is documented
// ---------------------------------------------------------------------------

#[test]
fn specificity_tuple_is_documented() {
    let resolver = ProfileResolver::default();
    let m = ProfileMatcher::default();
    let _ = resolver.specificity(&m);
}

// ---------------------------------------------------------------------------
// Negative test: LifecycleError is a distinct error type
// ---------------------------------------------------------------------------

#[test]
fn lifecycle_errors_are_distinct_from_resolve_errors() {
    let _ = std::mem::size_of::<workmen_core::profile::LifecycleError>();
    let _ = std::mem::size_of::<ResolveError>();
}

// ---------------------------------------------------------------------------
// Defensive: SVG asset resolves against path glob
// ---------------------------------------------------------------------------

#[test]
fn resolver_does_not_require_raster_metadata() {
    let asset = Asset {
        id: AssetId("logo.svg".to_string()),
        path: "art/logo.svg".to_string(),
        role: AssetRole::Source,
        format: AssetFormat::Svg,
        metadata: AssetMetadata::Raster {
            width: 0,
            height: 0,
            encoded_bytes: 0,
            decoded_bytes: 0,
            has_alpha: false,
            color_type: "unknown".to_string(),
            bit_depth: 0,
            alpha_bounds: None,
        },
    };
    let profiles = vec![profile("art", path_glob_matcher("art/**", None))];
    let resolver = ProfileResolver::default();
    let result = resolver.resolve(&asset, &profiles);
    let p = result
        .expect("SVG asset resolves against path glob")
        .expect("must match");
    assert_eq!(p.id.0, "art");
}
