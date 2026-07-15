//! Validation: Spec Diff and rule evaluation.
//!
//! `Validator::validate_asset` returns one `ValidationIssue` per
//! failed rule/platform. The plan says:
//! - "Return one ValidationIssue per failed rule/platform with
//!   actual/expected values and suggested next action. Do not
//!   collapse cross-platform failures."
//! - "Apply versioned exceptions only when rule, asset matcher,
//!   and unexpired date all match."

mod diff;
mod rules;

pub use diff::compute_spec_diff;
pub use rules::{RuleOutcome, Validator, validate_asset};

/// Validate one asset against one profile for one platform.
pub type ValidateResult = Vec<crate::model::ValidationIssue>;

/// Result of validating one asset against one profile on one
/// platform.
#[derive(Clone, Debug, PartialEq)]
pub struct AssetValidation {
    pub issues: Vec<crate::model::ValidationIssue>,
}
