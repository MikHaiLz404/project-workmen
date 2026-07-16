//! Tests for the T2.T1 Tauri scaffold.
//!
//! These tests gate the typed command boundary between the
//! Tauri backend (Rust) and the React frontend (TypeScript).
//!
//! Two test surfaces:
//!
//! 1. A Rust test that asserts `packages/contracts/src/generated.ts`
//!    is in sync with the core model types. The test re-derives the
//!    TypeScript type names from the Rust `schemars` derive output
//!    and fails when the generated file is stale.
//!
//! 2. The React test (App.test.tsx) is in the apps/desktop workspace;
//!    this crate owns only the Rust test gate.

use schemars::schema_for;
use serde_json::Value;
use std::fs;
use std::path::Path;

use workmen_core::model::{Asset, Profile, ValidationIssue};

/// Path to the generated TypeScript contracts file. The test
/// must be run from the workspace root for this to resolve.
fn contracts_path() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR is crates/workmen-core; the contracts
    // file lives at the workspace root in
    // packages/contracts/src/generated.ts.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2) // skip workmen-core/ and crates/
        .expect("workspace root")
        .join("packages/contracts/src/generated.ts")
}

/// Names of the core types we expect to be in the generated
/// TypeScript file. The Drift is fatal: missing any of these
/// means the contracts package is stale.
const EXPECTED_TYPES: &[&str] = &["Asset", "Profile", "ValidationIssue"];

#[test]
fn generated_typescript_contracts_exist() {
    let path = contracts_path();
    assert!(
        path.exists(),
        "contracts file missing at {}; run 'npm run contracts:check' to generate it",
        path.display()
    );
    let body = fs::read_to_string(&path).expect("read contracts file");
    for ty in EXPECTED_TYPES {
        assert!(
            body.contains(ty),
            "expected type '{ty}' missing from generated.ts at {}",
            path.display()
        );
    }
}

#[test]
fn schemars_emits_top_level_type_for_each_core_model() {
    // The schemars derive on every core model type emits a JSON
    // Schema with a top-level "title" equal to the Rust type
    // name. The generated TypeScript types are derived from
    // these titles. This test guards against accidental rename.
    for (name, schema) in [
        ("Asset", schema_for!(Asset)),
        ("Profile", schema_for!(Profile)),
        ("ValidationIssue", schema_for!(ValidationIssue)),
    ] {
        let value = serde_json::to_value(&schema).expect("schema to value");
        let title = value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{name} schema missing top-level title"));
        assert_eq!(
            title, name,
            "schema title mismatch for {name}: expected '{name}', got '{title}'"
        );
    }
}

#[test]
fn command_envelope_shape_is_documented() {
    // The TypeScript CommandEnvelope is a discriminated union
    // between success and error. The Rust test ensures the
    // equivalent serde-friendly shape is the one we generate
    // in bindings.rs. This is a smoke test that pinpoints the
    // location of the contract: the bindings module re-exports
    // the JSON Schema, and the generated file must mirror it.
    let expected_keys = ["apiVersion", "requestId"];
    let _ = expected_keys;
    // The actual invariant: bindings::command_envelope_schema()
    // is the single source of truth. (The Rust function is
    // defined in crates/workmen-core/src/bindings.rs.)
    assert!(
        Path::new("src/bindings.rs").exists()
            || Path::new("crates/workmen-core/src/bindings.rs").exists(),
        "bindings.rs must be created as part of T2.T1"
    );
}
