//! Header-only metadata decoders.
//!
//! These functions read just enough of an image file to extract
//! dimensions, format, and channel info. The full pixel buffer is
//! *not* retained: the `image` crate's `ImageReader` API supports a
//! header-only read via [`image::ImageReader::into_dimensions`] and
//! [`image::ImageReader::with_guessed_format`]. The decoded byte
//! estimate is computed with checked arithmetic so a 65535x65535
//! image does not silently overflow a `u64`.

use std::path::Path;

use image::ImageReader;

use crate::WorkmenError;
use crate::model::{PixelSize, Rect, ViewBox};

/// Metadata extracted from a raster (PNG / JPG / WebP) header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterMeta {
    pub width: u32,
    pub height: u32,
    /// On-disk encoded size. The scanner reports this verbatim.
    pub encoded_bytes: u64,
    /// Decoded pixel buffer size. Computed as
    /// `width * height * channels` with checked arithmetic so a
    /// 65535x65535 image does not overflow a `u64`.
    pub decoded_bytes: u64,
    /// True if the image has an alpha channel.
    pub has_alpha: bool,
    /// Color-type name. The `image` crate reports
    /// `ColorType::L8` / `Rgb8` / `Rgba8` / etc. We render those as
    /// human-readable strings rather than re-export the enum.
    pub color_type: String,
    /// Bit depth per channel. The `image` crate's `ColorType` does
    /// not carry bit depth directly, so we use the file extension +
    /// a best-effort default.
    pub bit_depth: u8,
    /// Optional alpha-bounds rectangle. Most images do not declare
    /// this; PNGs may carry a `tRNS` chunk that we could parse in
    /// a future revision. For now, always `None`.
    pub alpha_bounds: Option<Rect>,
}

/// Metadata extracted from a vector (SVG / contextual XML) file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VectorMeta {
    pub view_box: Option<ViewBox>,
    /// Raster preview targets this file is expected to render to.
    /// For now, always empty.
    pub raster_preview_targets: Vec<PixelSize>,
}

/// Decode raster metadata for `path`. Returns an error if the file
/// is not a recognized raster format or if reading the header fails.
pub fn decode_raster_metadata(path: &Path) -> Result<RasterMeta, WorkmenError> {
    let encoded_bytes = std::fs::metadata(path)
        .map_err(|e| WorkmenError::io(path, e))?
        .len();

    // The `ImageReader` API: open the file, guess the format, read
    // the dimensions. This does not decode the pixel buffer.
    let reader = ImageReader::open(path)
        .map_err(|e| WorkmenError::decode(path, format!("image crate open: {e}")))?
        .with_guessed_format()
        .map_err(|e| WorkmenError::decode(path, format!("image crate guess format: {e}")))?;
    let format = reader.format();
    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| WorkmenError::decode(path, format!("image crate dimensions: {e}")))?;

    // Compute the channel count and bit depth per format. JPG is
    // always RGB (no alpha). PNG and WebP may be RGB or RGBA.
    let (has_alpha, color_type, bit_depth) = match format {
        Some(image::ImageFormat::Png) => {
            // The `image` crate does not surface PNG color type from
            // the header alone; we use a sensible default. A future
            // revision could read the IHDR chunk directly.
            (true, "RGBA".to_string(), 8)
        }
        Some(image::ImageFormat::Jpeg) => (false, "RGB".to_string(), 8),
        Some(image::ImageFormat::WebP) => (true, "RGBA".to_string(), 8),
        Some(other) => (true, format!("{other:?}"), 8),
        None => {
            return Err(WorkmenError::decode(
                path,
                "unknown image format (cannot decode without extension)".to_string(),
            ));
        }
    };

    // Checked arithmetic for `decoded_bytes`. Each channel is 1
    // byte; an RGBA image has 4 channels.
    let channels: u64 = if has_alpha { 4 } else { 3 };
    let decoded_bytes = (width as u64)
        .checked_mul(height as u64)
        .and_then(|h| h.checked_mul(channels))
        .ok_or_else(|| {
            WorkmenError::decode(
                path,
                format!("decoded_byte estimate overflows u64: {width}x{height}x{channels}"),
            )
        })?;

    Ok(RasterMeta {
        width,
        height,
        encoded_bytes,
        decoded_bytes,
        has_alpha,
        color_type,
        bit_depth,
        alpha_bounds: None,
    })
}

/// Decode vector metadata for `path`. SVG is the only vector format
/// the scanner currently understands; other XML files (e.g.
/// AndroidVectorXml, AndroidAdaptiveIconXml) return a metadata with
/// an empty `view_box` and an empty `raster_preview_targets` so the
/// scanner can still record them.
pub fn decode_vector_metadata(path: &Path) -> Result<VectorMeta, WorkmenError> {
    // Read the whole file. SVG files are small (rarely > 64 KiB)
    // and we only need to find the `viewBox` attribute.
    let text = std::fs::read_to_string(path).map_err(|e| WorkmenError::io(path, e))?;

    // Naive `viewBox` parse: look for `viewBox="minx miny w h"` and
    // parse four signed integers. We do *not* pull in a real XML
    // parser; the scanner is read-only and the `viewBox` attribute
    // is the only field we need.
    let view_box = parse_svg_view_box(&text);

    Ok(VectorMeta {
        view_box,
        raster_preview_targets: Vec::new(),
    })
}

/// Extract `viewBox="x y w h"` from a snippet of SVG. Returns `None`
/// if the attribute is absent or malformed.
fn parse_svg_view_box(svg: &str) -> Option<ViewBox> {
    // Find the literal `viewBox=` (case-insensitive).
    let lower = svg.to_ascii_lowercase();
    let idx = lower.find("viewbox=")?;
    let after = &svg[idx + "viewBox=".len()..];
    // Skip whitespace and an opening quote (single or double).
    let after = after.trim_start();
    let after = after
        .strip_prefix('"')
        .or_else(|| after.strip_prefix('\''))?;
    // Read until the matching quote.
    let end = after.find('"').or_else(|| after.find('\''))?;
    let inner = &after[..end];
    let mut parts = inner.split_ascii_whitespace();
    let min_x: i32 = parts.next()?.parse().ok()?;
    let min_y: i32 = parts.next()?.parse().ok()?;
    let width: u32 = parts.next()?.parse().ok()?;
    let height: u32 = parts.next()?.parse().ok()?;
    Some(ViewBox {
        min_x,
        min_y,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn fresh_tempdir(label: &str) -> std::path::PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("workmen-scan-meta-{label}-{pid}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("create tempdir");
        dir
    }

    #[test]
    fn decode_raster_png_8x8() {
        let tmp = fresh_tempdir("raster-8");
        let path = tmp.join("x.png");
        let img: image::RgbaImage = ImageBuffer::from_fn(8, 8, |_x, _y| Rgba([255, 0, 0, 255]));
        img.save(&path).expect("write png");
        let meta = decode_raster_metadata(&path).expect("decode");
        assert_eq!(meta.width, 8);
        assert_eq!(meta.height, 8);
        assert!(meta.has_alpha);
        // 8 * 8 * 4 = 256
        assert_eq!(meta.decoded_bytes, 256);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn decode_raster_jpg_no_alpha() {
        let tmp = fresh_tempdir("raster-jpg");
        let path = tmp.join("x.jpg");
        let img: image::RgbaImage = ImageBuffer::from_fn(8, 8, |_x, _y| Rgba([0, 255, 0, 255]));
        let dyn_img = image::DynamicImage::ImageRgba8(img);
        let rgb = dyn_img.to_rgb8();
        rgb.save(&path).expect("write jpg");
        let meta = decode_raster_metadata(&path).expect("decode jpg");
        assert_eq!(meta.width, 8);
        // 8 * 8 * 3 = 192
        assert_eq!(meta.decoded_bytes, 192);
        assert!(!meta.has_alpha);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn decode_vector_svg_view_box() {
        let tmp = fresh_tempdir("vector-svg");
        let path = tmp.join("x.svg");
        std::fs::write(
            &path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
  <rect width="32" height="32" fill="black"/>
</svg>"#,
        )
        .expect("write svg");
        let meta = decode_vector_metadata(&path).expect("decode svg");
        let vb = meta.view_box.expect("viewBox present");
        assert_eq!(vb.min_x, 0);
        assert_eq!(vb.min_y, 0);
        assert_eq!(vb.width, 32);
        assert_eq!(vb.height, 32);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn decode_vector_svg_without_view_box_returns_none() {
        let tmp = fresh_tempdir("vector-svg-no-vb");
        let path = tmp.join("x.svg");
        std::fs::write(
            &path,
            r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <rect/>
</svg>"#,
        )
        .expect("write svg");
        let meta = decode_vector_metadata(&path).expect("decode svg");
        assert!(meta.view_box.is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
