//! Asset domain model.
//!
//! An `Asset` is a single file inside a Workmen project that the project
//! scanner has recognised as either game art (raster or vector) or as
//! contextual metadata that exists to serve that art (iOS asset catalog,
//! Android vector / adaptive-icon XML, runtime asset manifests, and so on).
//!
//! Every asset has:
//!   * an [`AssetId`] (a stable opaque string)
//!   * a normalized project-relative [`Asset::path`]
//!   * an [`AssetRole`] from the six-role vocabulary (Source, Runtime,
//!     Derived, Mirror Target, Excluded, Unclassified)
//!   * an [`AssetFormat`] the scanner inferred from structural markers
//!   * a typed [`AssetMetadata`] block carrying the dimensions / preview
//!     targets the validator needs without keeping image bytes around

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Opaque stable identifier for an [`Asset`] inside a single Workmen
/// project. Stores a UTF-8 string under the hood; serializes as a bare
/// string via `#[serde(transparent)]`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AssetId(pub String);

/// Opaque stable identifier for an [`AssetFamily`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FamilyId(pub String);

/// The six-role vocabulary for asset classification.
///
/// Roles are deliberately distinct, even when some have empty user
/// populations today. Downstream tasks add new evidence for `Derived` and
/// `Mirror Target`; modelling them now keeps the on-disk schema stable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AssetRole {
    /// The original, hand-authored art for an asset family.
    Source,
    /// The build output that ships with the game.
    Runtime,
    /// Anything generated from a Source by Workmen (previews, mirrored
    /// copies, packed atlases, ...).
    Derived,
    /// A copy of a Runtime asset created for a specific target platform
    /// (web/iOS/Android) by a separate export pipeline. Always linked back
    /// to one Runtime asset.
    MirrorTarget,
    /// An asset that exists but is intentionally out of scope for the
    /// active profile (deprecated, rejected, license-blocked, ...).
    Excluded,
    /// A scanned file that we could not classify with the available
    /// evidence. Held in the Unclassified Queue until the user resolves it.
    Unclassified,
}

/// Asset format recorded by the scanner.
///
/// The closed variants cover the formats Workmen understands end-to-end.
/// [`AssetFormat::Other`] is an escape hatch for formats the scanner can
/// recognise structurally but not yet inspect (e.g. a future HDR format);
/// downstream code must handle it explicitly.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum AssetFormat {
    Png,
    Jpg,
    WebP,
    Svg,

    /// iOS asset catalog `Contents.json` files.
    IosAssetCatalogJson,
    /// Android VectorDrawable XML.
    AndroidVectorXml,
    /// Android adaptive icon XML (background / foreground layers).
    AndroidAdaptiveIconXml,
    /// JavaScript asset-manifest understood by the runtime loader.
    RuntimeManifestJs,

    /// A recognised structural marker but a format Workmen does not yet
    /// understand natively. Carries the format marker for diagnostic use.
    Other(String),
}

impl AssetFormat {
    /// `true` when the format is raster art that Workmen can decode and
    /// re-encode. SVG and the contextual metadata formats return `false`.
    pub fn is_raster(&self) -> bool {
        matches!(self, Self::Png | Self::Jpg | Self::WebP)
    }

    /// `true` when the format carries contextual metadata for a runtime
    /// asset rather than the asset itself.
    pub fn is_contextual(&self) -> bool {
        matches!(
            self,
            Self::IosAssetCatalogJson
                | Self::AndroidVectorXml
                | Self::AndroidAdaptiveIconXml
                | Self::RuntimeManifestJs
        )
    }
}

/// A pixel rectangle used to record `alphaBounds` for raster assets.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A vector view box in user-space units.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ViewBox {
    pub min_x: i32,
    pub min_y: i32,
    pub width: u32,
    pub height: u32,
}

/// A requested raster preview target (e.g. `64x64`, `128x128`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
}

/// Per-asset metadata block.
///
/// Uses an internally tagged representation so the JSON keeps a `kind`
/// discriminator. The variant itself selects which field set is present;
/// the inner fields use camelCase via the explicit per-field serde
/// renames (a single `#[serde(rename_all = "camelCase")]` on the enum
/// would collide with the `tag = "kind"` discriminator, so we are
/// explicit per field).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum AssetMetadata {
    /// Metadata for raster (PNG/JPG/WebP) assets.
    #[serde(rename = "raster")]
    Raster {
        width: u32,
        height: u32,
        #[serde(rename = "encodedBytes")]
        encoded_bytes: u64,
        #[serde(rename = "decodedBytes")]
        decoded_bytes: u64,
        #[serde(rename = "hasAlpha")]
        has_alpha: bool,
        #[serde(rename = "colorType")]
        color_type: String,
        #[serde(rename = "bitDepth")]
        bit_depth: u8,
        #[serde(rename = "alphaBounds")]
        alpha_bounds: Option<Rect>,
    },
    /// Metadata for vector (SVG and contextual XML) assets.
    #[serde(rename = "vector")]
    Vector {
        #[serde(rename = "viewBox")]
        view_box: Option<ViewBox>,
        #[serde(rename = "rasterPreviewTargets")]
        raster_preview_targets: Vec<PixelSize>,
    },
}

impl AssetMetadata {
    /// Reported pixel width for raster assets. Vector assets report `None`.
    pub fn width(&self) -> Option<u32> {
        match self {
            Self::Raster { width, .. } => Some(*width),
            Self::Vector { .. } => None,
        }
    }

    /// Reported pixel height for raster assets. Vector assets report `None`.
    pub fn height(&self) -> Option<u32> {
        match self {
            Self::Raster { height, .. } => Some(*height),
            Self::Vector { .. } => None,
        }
    }
}

/// A single recognised file inside a Workmen project.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: AssetId,
    pub path: String,
    pub role: AssetRole,
    pub format: AssetFormat,
    pub metadata: AssetMetadata,
}

impl Asset {
    /// The normalized project-relative path of this asset.
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// A grouping of assets that together represent the same logical art
/// concept across formats and platforms.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssetFamily {
    pub id: FamilyId,
    pub name: String,
    /// A handful of representative paths used to render the family in the
    /// Validation Console.
    pub representative_paths: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_role_labels_match_design() {
        // Mirror of the contract test, expressed as a unit test so changes
        // to the rename rule fail close to the type definition.
        let parsed: AssetRole = serde_json::from_str("\"mirrorTarget\"").unwrap();
        assert_eq!(parsed, AssetRole::MirrorTarget);
        let parsed: AssetRole = serde_json::from_str("\"unclassified\"").unwrap();
        assert_eq!(parsed, AssetRole::Unclassified);
    }

    #[test]
    fn asset_format_other_round_trips() {
        let format = AssetFormat::Other("heif".to_string());
        let json = serde_json::to_string(&format).unwrap();
        let back: AssetFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(back, format);
    }

    #[test]
    fn asset_metadata_raster_uses_camel_case_inner_fields() {
        let meta = AssetMetadata::Raster {
            width: 8,
            height: 8,
            encoded_bytes: 128,
            decoded_bytes: 256,
            has_alpha: true,
            color_type: "RGBA".into(),
            bit_depth: 8,
            alpha_bounds: None,
        };
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["kind"], "raster");
        assert_eq!(json["encodedBytes"], 128);
        assert_eq!(json["decodedBytes"], 256);
        assert_eq!(json["hasAlpha"], true);
        assert_eq!(json["colorType"], "RGBA");
        assert_eq!(json["bitDepth"], 8);
        assert!(json["alphaBounds"].is_null());
    }

    #[test]
    fn raster_metadata_dimensions_accessors() {
        let raster = AssetMetadata::Raster {
            width: 64,
            height: 32,
            encoded_bytes: 1,
            decoded_bytes: 1,
            has_alpha: false,
            color_type: "RGB".into(),
            bit_depth: 8,
            alpha_bounds: None,
        };
        assert_eq!(raster.width(), Some(64));
        assert_eq!(raster.height(), Some(32));

        let vector = AssetMetadata::Vector {
            view_box: None,
            raster_preview_targets: vec![],
        };
        assert_eq!(vector.width(), None);
        assert_eq!(vector.height(), None);
    }
}
