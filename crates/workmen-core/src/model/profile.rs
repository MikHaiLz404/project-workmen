//! Profile domain model.
//!
//! A `Profile` describes a contract that a subset of a project's assets
//! must satisfy: how to identify the assets (matchers), what naming
//! scheme they follow (naming rules), which pairs of source and runtime
//! paths we know about (source/runtime relationships), what exceptions
//! we have agreed to tolerate for this revision, and what budget each
//! shipping platform exposes.
//!
//! Profiles are versioned: every change increments `profile_revision`.
use super::asset::AssetRole;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Opaque stable identifier for a [`Profile`].
#[derive(
    Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct ProfileId(pub String);

/// Target platform a Profile budgets for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    Web,
    Ios,
    Android,
}

/// A predicate that selects assets into a Profile.
///
/// Each field is optional. An empty [`ProfileMatcher`] matches every
/// asset — usually a bug, but legitimate for the "default" Profile.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMatcher {
    /// Project-relative glob (`.workmen`-style anchored to project root).
    #[serde(rename = "pathGlob")]
    pub path_glob: Option<String>,
    /// Regex-style naming pattern (the syntax T5 will pin down).
    #[serde(rename = "namingPattern")]
    pub naming_pattern: Option<String>,
    /// Extension without the leading dot.
    pub extension: Option<String>,
    /// Optional role filter; `None` accepts any role.
    #[serde(rename = "assetRole")]
    pub asset_role: Option<AssetRole>,
}

/// A naming rule applied before raising naming violations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NamingRule {
    pub pattern: String,
    pub description: String,
}

/// Declared edge between a Source asset and a Runtime asset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceRuntimeRelationship {
    #[serde(rename = "sourcePath")]
    pub source_path: String,
    #[serde(rename = "runtimePath")]
    pub runtime_path: String,
}

/// A bounded, auditable exception to a Profile rule.
///
/// `expires_at` is an ISO-8601 timestamp string (`chrono` is intentionally
/// avoided for this task to keep the dependency surface small — future
/// revisions will replace `String` with `chrono::DateTime<Utc>`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileException {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    #[serde(rename = "assetMatcher")]
    pub asset_matcher: ProfileMatcher,
    pub reason: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<String>,
}

/// Alpha channel policy for a given platform budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AlphaPolicy {
    Required,
    Optional,
    Forbidden,
}

/// Compression codec negotiated for the runtime asset on a platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Compression {
    None,
    Png,
    /// WebP. Note: serializes to `"webP"` (camelCase of the variant name),
    /// not the all-lowercase `"webp"` that some consumers may expect. This
    /// is deliberate for consistency with the other camelCase variants
    /// in this enum.
    WebP,
}

/// Whether the platform requires power-of-two texture dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PotNpot {
    /// Strictly power-of-two. Non-POT assets are validation errors.
    PowerOfTwo,
    /// Any dimensions are accepted.
    Any,
}

/// A platform-specific budget used by the validator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformBudget {
    pub platform: Platform,
    #[serde(rename = "maxTextureWidth")]
    pub max_texture_width: u32,
    #[serde(rename = "maxTextureHeight")]
    pub max_texture_height: u32,
    #[serde(rename = "maxEncodedBytes")]
    pub max_encoded_bytes: u64,
    #[serde(rename = "maxDecodedBytes")]
    pub max_decoded_bytes: u64,
    /// Free-form string describing the color space (e.g. `"sRGB"`).
    #[serde(rename = "colorSpace")]
    pub color_space: String,
    #[serde(rename = "alphaPolicy")]
    pub alpha_policy: AlphaPolicy,
    pub compression: Compression,
    #[serde(rename = "potNpot")]
    pub pot_npot: PotNpot,
}

impl PlatformBudget {
    /// Construct a reasonable default budget for a given platform. The
    /// numbers here are placeholders for the M2 budget tuning; they exist
    /// so the contract test has a real value to serialize.
    pub fn default_for(platform: Platform) -> Self {
        match platform {
            Platform::Web => Self {
                platform,
                max_texture_width: 4096,
                max_texture_height: 4096,
                max_encoded_bytes: 8 * 1024 * 1024,
                max_decoded_bytes: 64 * 1024 * 1024,
                color_space: "sRGB".to_string(),
                alpha_policy: AlphaPolicy::Optional,
                compression: Compression::WebP,
                pot_npot: PotNpot::Any,
            },
            Platform::Ios => Self {
                platform,
                max_texture_width: 4096,
                max_texture_height: 4096,
                max_encoded_bytes: 8 * 1024 * 1024,
                max_decoded_bytes: 64 * 1024 * 1024,
                color_space: "sRGB".to_string(),
                alpha_policy: AlphaPolicy::Optional,
                compression: Compression::Png,
                pot_npot: PotNpot::PowerOfTwo,
            },
            Platform::Android => Self {
                platform,
                max_texture_width: 4096,
                max_texture_height: 4096,
                max_encoded_bytes: 8 * 1024 * 1024,
                max_decoded_bytes: 64 * 1024 * 1024,
                color_space: "sRGB".to_string(),
                alpha_policy: AlphaPolicy::Optional,
                compression: Compression::WebP,
                pot_npot: PotNpot::Any,
            },
        }
    }
}

/// Lifecycle state of a [`Profile`].
///
/// * `Draft` — the profile is being edited; not authoritative.
/// * `Active` — the profile is the contract of record.
/// * `Locked` — the profile cannot be edited; bumps require explicit
///   unlock with a non-empty reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProfileState {
    Draft,
    Active,
    Locked,
}

/// The Workmen profile contract. See module docs for field semantics.
///
/// The `schema_version` field is deserialized through a custom function
/// that enforces the supported version. Unknown versions fail at
/// deserialization time (rather than at first use) so a stale cache or
/// stale in-memory copy never silently disagrees with the on-disk file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(
        rename = "schemaVersion",
        deserialize_with = "deserialize_schema_version"
    )]
    pub schema_version: u32,
    pub id: ProfileId,
    pub profile_revision: u32,
    pub state: ProfileState,
    pub matchers: Vec<ProfileMatcher>,
    pub naming_rules: Vec<NamingRule>,
    pub source_runtime: Vec<SourceRuntimeRelationship>,
    pub exceptions: Vec<ProfileException>,
    pub budgets: Vec<PlatformBudget>,
}

fn deserialize_schema_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u32::deserialize(deserializer)?;
    if value == Profile::SUPPORTED_SCHEMA_VERSION {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "unsupported profile schema version {value}; Workmen only understands version {}",
            Profile::SUPPORTED_SCHEMA_VERSION
        )))
    }
}

impl Profile {
    /// The schema version of profile payloads this crate understands.
    /// Bumped alongside any backward-incompatible field change.
    pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

    /// Convenience helper: did the model version gate accept this schema
    /// version? Today the only supported value is 1.
    pub fn accepts_schema_version(version: u32) -> bool {
        version == Self::SUPPORTED_SCHEMA_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_serializes_to_camel_case() {
        let v = serde_json::to_value(Platform::Ios).unwrap();
        assert_eq!(v, serde_json::json!("ios"));
        let v = serde_json::to_value(Platform::Android).unwrap();
        assert_eq!(v, serde_json::json!("android"));
    }

    #[test]
    fn profile_state_serializes_to_camel_case() {
        let v = serde_json::to_value(ProfileState::Locked).unwrap();
        assert_eq!(v, serde_json::json!("locked"));
    }

    #[test]
    fn alpha_policy_and_compression_and_pot_npot_serializes_to_camel_case() {
        assert_eq!(
            serde_json::to_value(AlphaPolicy::Required).unwrap(),
            serde_json::json!("required")
        );
        assert_eq!(
            serde_json::to_value(Compression::WebP).unwrap(),
            serde_json::json!("webP")
        );
        assert_eq!(
            serde_json::to_value(PotNpot::PowerOfTwo).unwrap(),
            serde_json::json!("powerOfTwo")
        );
    }

    #[test]
    fn default_budget_covers_all_three_platforms() {
        let web = PlatformBudget::default_for(Platform::Web);
        let ios = PlatformBudget::default_for(Platform::Ios);
        let android = PlatformBudget::default_for(Platform::Android);
        assert_eq!(web.platform, Platform::Web);
        assert_eq!(ios.platform, Platform::Ios);
        assert_eq!(android.platform, Platform::Android);
        // Sanity: each budget has a non-empty color space.
        assert!(!web.color_space.is_empty());
        assert!(!ios.color_space.is_empty());
        assert!(!android.color_space.is_empty());
    }

    #[test]
    fn supported_schema_version_is_one() {
        assert_eq!(Profile::SUPPORTED_SCHEMA_VERSION, 1);
        assert!(Profile::accepts_schema_version(1));
        assert!(!Profile::accepts_schema_version(2));
        assert!(!Profile::accepts_schema_version(99));
    }
}
