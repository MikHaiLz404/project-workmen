//! Workmen core domain contract tests (Task 2).
//!
//! These tests gate the [`workmen_core::model`] module: they assert stable
//! JSON serialization values for the canonical domain contracts, verify that
//! unknown `schemaVersion` values are rejected, prove that every public type
//! satisfies the [`schemars::JsonSchema`] contract, and lock the generated
//! JSON Schemas against drift from the checked-in files under
//! `schemas/workmen-*.schema.json`.

use schemars::schema_for;
use serde_json::{Value, json};

use workmen_core::model::{
    Asset, AssetFamily, AssetFormat, AssetMetadata, AssetRole, OperationEvent, OperationKind,
    Platform, PlatformBudget, Profile, ProfileId, ProfileMatcher, ProfileState, Severity,
    SourceRuntimeRelationship, SpecDiff, ValidationIssue,
};
use workmen_core::schema::{profile_schema, project_schema};

#[test]
fn asset_role_serializes_to_camel_case_values() {
    // The mirrorTarget / unclassified / runtime / derived / excluded / source
    // labels are part of the on-disk contract. Anyone renaming the variants
    // or removing `rename_all = "camelCase"` breaks the Workmen file format.
    let payload = serde_json::to_value([
        AssetRole::Source,
        AssetRole::Runtime,
        AssetRole::Derived,
        AssetRole::MirrorTarget,
        AssetRole::Excluded,
        AssetRole::Unclassified,
    ])
    .expect("AssetRole serializes");

    assert_eq!(
        payload,
        json!([
            "source",
            "runtime",
            "derived",
            "mirrorTarget",
            "excluded",
            "unclassified"
        ])
    );
}

#[test]
fn asset_role_rejects_unknown_variant_string() {
    // Defensive: nothing should produce "Unknown" / "Other" role names; they
    // would silently pass serialization. This stays as a negative check that
    // any future "Other" variant comes with an explicit decision.
    let parsed: Result<AssetRole, _> = serde_json::from_value(json!("ghostRole"));
    assert!(parsed.is_err(), "expected unknown AssetRole to fail");
}

#[test]
fn asset_round_trips_through_json() {
    let asset = Asset {
        id: workmen_core::model::AssetId("asset-001".to_string()),
        path: "assets/player/sprite.png".to_string(),
        role: AssetRole::Source,
        format: AssetFormat::Png,
        metadata: AssetMetadata::Raster {
            width: 128,
            height: 128,
            encoded_bytes: 4096,
            decoded_bytes: 65536,
            has_alpha: true,
            color_type: "RGBA".to_string(),
            bit_depth: 8,
            alpha_bounds: None,
        },
    };

    let serialized = serde_json::to_value(&asset).expect("serialize Asset");
    let parsed: Asset = serde_json::from_value(serialized.clone()).expect("round-trip Asset");

    assert_eq!(parsed, asset);

    // camelCase enforcement for Asset's multi-word fields.
    let obj = serialized.as_object().expect("Asset serializes to object");
    for key in obj.keys() {
        assert!(
            !key.contains('_'),
            "Asset JSON keys must be camelCase, found `{key}`"
        );
    }
    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("path"));
    assert!(obj.contains_key("role"));
    assert!(obj.contains_key("format"));
    assert!(obj.contains_key("metadata"));
}

#[test]
fn asset_family_and_id_serialize_transparently() {
    let family = AssetFamily {
        id: workmen_core::model::FamilyId("family-001".to_string()),
        name: "player-coin".to_string(),
        representative_paths: vec!["assets/coins/gold.png".to_string()],
    };
    let serialized = serde_json::to_value(&family).expect("serialize AssetFamily");
    // FamilyId is `#[serde(transparent)]` so the `id` field serializes as a
    // bare string, not as `{"0": "..."}`.
    assert_eq!(serialized["id"], json!("family-001"));
    assert_eq!(serialized["name"], json!("player-coin"));
    assert_eq!(
        serialized["representativePaths"],
        json!(["assets/coins/gold.png"])
    );

    let round: AssetFamily = serde_json::from_value(serialized).expect("round-trip");
    assert_eq!(round, family);
}

#[test]
fn profile_uses_camel_case_field_names() {
    let profile = Profile {
        schema_version: 1,
        id: ProfileId("default-web".to_string()),
        profile_revision: 1,
        state: ProfileState::Draft,
        matchers: vec![ProfileMatcher {
            path_glob: Some("assets/**/*.png".to_string()),
            naming_pattern: Some("{name}@{scale}x".to_string()),
            extension: Some("png".to_string()),
            asset_role: Some(AssetRole::Source),
        }],
        naming_rules: vec![],
        source_runtime: vec![SourceRuntimeRelationship {
            source_path: "assets/coin@1x.png".to_string(),
            runtime_path: "public/img/coin.png".to_string(),
        }],
        exceptions: vec![],
        budgets: vec![PlatformBudget::default_for(Platform::Web)],
    };

    let serialized = serde_json::to_value(&profile).expect("serialize Profile");
    let obj = serialized.as_object().expect("Profile is an object");

    assert!(
        obj.contains_key("schemaVersion"),
        "Profile must use `schemaVersion` (camelCase) — got keys {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("profileRevision"));
    assert!(obj.contains_key("state"));
    assert!(obj.contains_key("matchers"));
    assert!(obj.contains_key("namingRules"));
    assert!(obj.contains_key("sourceRuntime"));
    assert!(obj.contains_key("exceptions"));
    assert!(obj.contains_key("budgets"));
    for key in obj.keys() {
        assert!(
            !key.contains('_'),
            "Profile keys must be camelCase; found `{key}`"
        );
    }
    assert_eq!(obj["schemaVersion"], json!(1));
    assert_eq!(obj["profileRevision"], json!(1));
    assert_eq!(obj["matchers"][0]["pathGlob"], json!("assets/**/*.png"));
    assert_eq!(
        obj["matchers"][0]["namingPattern"],
        json!("{name}@{scale}x")
    );
    assert_eq!(obj["matchers"][0]["assetRole"], json!("source"));
    assert_eq!(
        obj["sourceRuntime"][0]["sourcePath"],
        json!("assets/coin@1x.png")
    );
    assert_eq!(
        obj["sourceRuntime"][0]["runtimePath"],
        json!("public/img/coin.png")
    );
}

#[test]
fn profile_rejects_unknown_schema_version() {
    // The only acceptable schemaVersion today is `1`. A future migration must
    // land before the file format itself changes.
    let payload = json!({
        "schemaVersion": 99,
        "id": "default-web",
        "profileRevision": 1,
        "state": "draft",
        "matchers": [],
        "namingRules": [],
        "sourceRuntime": [],
        "exceptions": [],
        "budgets": []
    });

    let parsed: Result<Profile, _> = serde_json::from_value(payload);
    assert!(
        parsed.is_err(),
        "Profile with schemaVersion=99 must be rejected"
    );
}

#[test]
fn severity_uses_lowercase_strings() {
    let payload = serde_json::to_value([Severity::Error, Severity::Warning, Severity::Info])
        .expect("Severity serializes");
    assert_eq!(payload, json!(["error", "warning", "info"]));
}

#[test]
fn spec_diff_carries_required_fields() {
    let diff = SpecDiff {
        rule_id: "texture.maxWidth".to_string(),
        profile_id: ProfileId("default-web".to_string()),
        expected: json!(2048),
        actual: json!(4096),
        platform: Some(Platform::Web),
        severity: Severity::Error,
        suggested_action: "downscale asset to <= 2048px wide".to_string(),
    };

    let serialized = serde_json::to_value(&diff).expect("serialize SpecDiff");
    assert_eq!(serialized["ruleId"], json!("texture.maxWidth"));
    assert_eq!(serialized["profileId"], json!("default-web"));
    assert_eq!(serialized["platform"], json!("web"));
    assert_eq!(serialized["severity"], json!("error"));
    assert_eq!(serialized["expected"], json!(2048));
    assert_eq!(serialized["actual"], json!(4096));
    assert_eq!(
        serialized["suggestedAction"],
        json!("downscale asset to <= 2048px wide")
    );

    let issue = ValidationIssue {
        asset_path: "assets/player.png".to_string(),
        diff,
    };
    let issue_json = serde_json::to_value(&issue).expect("serialize ValidationIssue");
    assert_eq!(issue_json["assetPath"], json!("assets/player.png"));
    assert_eq!(issue_json["diff"]["ruleId"], json!("texture.maxWidth"));
}

#[test]
fn operation_event_uses_expected_kind_strings() {
    let event = OperationEvent {
        timestamp: "2026-07-13T00:00:00Z".to_string(),
        kind: OperationKind::Validate,
        asset_path: Some("assets/player.png".to_string()),
        message: "2 issues".to_string(),
        input_hash: Some("blake3:abc".to_string()),
        output_hash: None,
        duration_ms: Some(12),
    };
    let serialized = serde_json::to_value(&event).expect("serialize OperationEvent");
    assert_eq!(serialized["kind"], json!("validate"));
    assert_eq!(serialized["assetPath"], json!("assets/player.png"));
    assert_eq!(serialized["durationMs"], json!(12));
    assert_eq!(serialized["timestamp"], json!("2026-07-13T00:00:00Z"));
}

#[test]
fn asset_and_profile_derive_json_schema() {
    // Compile-time check: if these types stop implementing JsonSchema the
    // test stops compiling.
    let _asset_schema = schema_for!(Asset);
    let _family_schema = schema_for!(AssetFamily);
    let _profile_schema = schema_for!(Profile);
    let _budget_schema = schema_for!(PlatformBudget);
    let _diff_schema = schema_for!(SpecDiff);
    let _issue_schema = schema_for!(ValidationIssue);
    let _event_schema = schema_for!(OperationEvent);
}

#[test]
fn locked_project_schemas_match_checked_in_files() {
    // The contract test owns schema generation. If you intentionally change
    // the on-disk format, regenerate with `cargo insta accept` and commit
    // the updated snapshot files.
    let project = project_schema();
    let profile = profile_schema();

    let project_json = serde_json::to_value(&project).expect("serialize project schema");
    let profile_json = serde_json::to_value(&profile).expect("serialize profile schema");

    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workmen-core lives at crates/workmen-core");
    let schemas_dir = workspace_root.join("schemas");

    let project_path = schemas_dir.join("workmen-project.schema.json");
    let profile_path = schemas_dir.join("workmen-profile.schema.json");

    if !project_path.exists() || !profile_path.exists() {
        // The checked-in JSON Schemas are the source of truth. If they are
        // missing, fail with a clear message: the test will not silently
        // auto-write them, because doing so would let a fresh clone with
        // the wrong files self-correct and pass CI. To regenerate, run
        // `cargo run -p workmen-cli -- generate-schemas` (added in T7).
        panic!(
            "checked-in schema files are missing: {} and/or {}. \
             regenerate with `cargo run -p workmen-cli -- generate-schemas` \
             and commit the result.",
            project_path.display(),
            profile_path.display()
        );
    }

    // Gate: both files must be tracked in the git index. A future
    // contributor who deletes the files from the working tree (or
    // untracks them) would otherwise pass this test on the first run
    // because the auto-write branch (now removed) would silently
    // recreate them. `git ls-files --error-unmatch` exits non-zero when
    // any listed path is not in the index.
    for path in [&project_path, &profile_path] {
        let rel = path
            .strip_prefix(workspace_root)
            .expect("schema path is inside the workspace");
        let status = std::process::Command::new("git")
            .args(["ls-files", "--error-unmatch", "--"])
            .arg(rel)
            .current_dir(workspace_root)
            .status()
            .expect("git ls-files must run in CI; install git or set --workspace-manifest-path");
        assert!(
            status.success(),
            "{} is not tracked by git. Re-add it with `git add {}` so the drift gate can rely on the index.",
            rel.display(),
            rel.display()
        );
    }

    let on_disk_project = std::fs::read_to_string(&project_path).expect("read project schema");
    let on_disk_profile = std::fs::read_to_string(&profile_path).expect("read profile schema");

    let on_disk_project_json: Value = serde_json::from_str(&on_disk_project).expect("parse");
    let on_disk_profile_json: Value = serde_json::from_str(&on_disk_profile).expect("parse");

    assert_eq!(
        on_disk_project_json, project_json,
        "generated project schema drifted from schemas/workmen-project.schema.json"
    );
    assert_eq!(
        on_disk_profile_json, profile_json,
        "generated profile schema drifted from schemas/workmen-profile.schema.json"
    );
}

#[test]
fn model_module_exposes_expected_public_surface() {
    // Smoke test that the re-exports planned in `lib.rs` actually land on
    // the public API surface used by future tasks.
    let _: AssetRole = AssetRole::Unclassified;
    let _: ProfileState = ProfileState::Locked;
    let _: OperationKind = OperationKind::Mirror;
    let _: AssetFormat = AssetFormat::Other("custom".to_string());
}
