// SPDX-License-Identifier: MIT OR Apache-2.0
//! Generated JSON Schemas.
//!
//! The checked-in JSON Schemas under `schemas/workmen-*.schema.json` are
//! produced from the Rust domain types via [`schemars`]. They are tested
//! for drift in `tests/contracts.rs`, which is the contract gate for
//! every change to a public domain type.
use schemars::schema::RootSchema;
use schemars::schema_for;

/// JSON Schema for the canonical Workmen project file.
///
/// Today the on-disk project shape is dominated by [`Profile`] (the
/// project file references exactly one active Profile). Once Task 3
/// (project root + config) lands a distinct project-only structure this
/// schema will diverge from the profile schema.
pub fn project_schema() -> RootSchema {
    schema_for!(crate::model::profile::Profile)
}

/// JSON Schema for the canonical Workmen Profile file.
pub fn profile_schema() -> RootSchema {
    schema_for!(crate::model::profile::Profile)
}
