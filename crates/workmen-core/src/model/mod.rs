//! Canonical Workmen domain model.
//!
//! Every Rust type that crosses a process boundary in Workmen lives here:
//! project, profile, asset, scan results, validation issues, and operation
//! events. This module is the single source of truth; the JSON Schemas under
//! `schemas/workmen-*.schema.json` are generated from these types.

pub mod asset;
pub mod operation;
pub mod profile;
pub mod validation;

pub use asset::{
    Asset, AssetFamily, AssetFormat, AssetId, AssetMetadata, AssetRole, FamilyId, PixelSize, Rect,
    ViewBox,
};
pub use operation::{OperationEvent, OperationKind};
pub use profile::{
    AlphaPolicy, Compression, NamingRule, Platform, PlatformBudget, PotNpot, Profile,
    ProfileException, ProfileId, ProfileMatcher, ProfileState, SourceRuntimeRelationship,
};
pub use validation::{Severity, SpecDiff, ValidationIssue};
