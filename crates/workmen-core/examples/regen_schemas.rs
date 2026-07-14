//! One-shot schema regenerator. Run with `cargo run --example regen_schemas`.
//!
//! This regenerates `schemas/workmen-{project,profile}.schema.json` from the
//! current Rust types. Used by T2 follow-up work and (in the future) by a
//! planned `workmen generate-schemas` CLI command (T7). For now it is a
//! developer-only example, not a published artifact.
use schemars::schema_for;
use std::path::PathBuf;
use workmen_core::model::profile::Profile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project = schema_for!(Profile);
    let profile = schema_for!(Profile);
    let project_json = serde_json::to_string_pretty(&project)?;
    let profile_json = serde_json::to_string_pretty(&profile)?;

    // Resolve the workspace root from CARGO_MANIFEST_DIR (.. from workmen-core).
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or("workmen-core must live at crates/workmen-core")?;
    let schemas_dir = workspace_root.join("schemas");
    std::fs::create_dir_all(&schemas_dir)?;

    let project_path = schemas_dir.join("workmen-project.schema.json");
    let profile_path = schemas_dir.join("workmen-profile.schema.json");

    std::fs::write(&project_path, format!("{project_json}\n"))?;
    std::fs::write(&profile_path, format!("{profile_json}\n"))?;
    println!(
        "regenerated {} and {}",
        project_path.display(),
        profile_path.display()
    );
    Ok(())
}
