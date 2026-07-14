//! Asset-format classification.
//!
//! `classify_format` returns the [`AssetFormat`] for a path. The
//! classifier uses *both* the file extension and structural markers
//! (e.g. the leading bytes of an XML or JSON file) so a `Contents.json`
//! in a random subdirectory that does not look like an asset catalog
//! surfaces as [`AssetFormat::Other`] rather than as an iOS context.

use std::fs;
use std::io::Read;
use std::path::Path;

use crate::model::AssetFormat;

/// Inspect the first 4 KiB of `path` to discover its content prefix.
/// Returns an empty `Vec` if the file is empty or unreadable; callers
/// must handle that case (e.g. by emitting a `DecodeError`).
fn read_prefix(path: &Path) -> Vec<u8> {
    const PREFIX_BYTES: usize = 4096;
    let Ok(mut f) = fs::File::open(path) else {
        return Vec::new();
    };
    let mut buf = vec![0u8; PREFIX_BYTES];
    let Ok(n) = f.read(&mut buf) else {
        return Vec::new();
    };
    buf.truncate(n);
    buf
}

/// Strip leading whitespace and look at the first non-whitespace byte.
fn first_non_whitespace(prefix: &[u8]) -> Option<u8> {
    prefix.iter().find(|b| !b.is_ascii_whitespace()).copied()
}

/// True iff the prefix starts with a JSON `{` after optional whitespace.
fn looks_like_json(prefix: &[u8]) -> bool {
    first_non_whitespace(prefix) == Some(b'{')
}

/// True iff the prefix starts with `<vector` after optional whitespace
/// and an XML prolog.
fn looks_like_vector_drawable(prefix: &[u8]) -> bool {
    // Strip an optional `<?xml ...?>` prolog before checking.
    let body = if prefix.starts_with(b"<?xml") {
        match prefix.windows(5).position(|w| w == b"?>") {
            Some(end) => &prefix[end + 2..],
            None => prefix,
        }
    } else {
        prefix
    };
    body.iter().find(|b| !b.is_ascii_whitespace()).copied() == Some(b'<')
        && body.windows(7).any(|w| w.eq_ignore_ascii_case(b"<vector"))
}

/// True iff the prefix starts with `<adaptive-icon` after an optional
/// XML prolog.
fn looks_like_adaptive_icon(prefix: &[u8]) -> bool {
    let body = if prefix.starts_with(b"<?xml") {
        match prefix.windows(5).position(|w| w == b"?>") {
            Some(end) => &prefix[end + 2..],
            None => prefix,
        }
    } else {
        prefix
    };
    body.iter().find(|b| !b.is_ascii_whitespace()).copied() == Some(b'<')
        && body
            .windows(14)
            .any(|w| w.eq_ignore_ascii_case(b"<adaptive-icon"))
}

/// True iff the prefix looks like JavaScript with the `__WORKMEN__`
/// marker the runtime asset-manifest is expected to declare.
fn looks_like_asset_manifest(prefix: &[u8]) -> bool {
    // Allow the manifest to start with a comment block; the marker
    // may appear after a `/*` or `//` line.
    std::str::from_utf8(prefix)
        .ok()
        .map(|s| s.contains("__WORKMEN__"))
        .unwrap_or(false)
}

/// Classify a path. The path's existence and contents are *not*
/// required: pure extension-based classification is the fast path
/// for the common cases (PNG/JPG/WebP/SVG). The contextual-metadata
/// formats require reading the file prefix.
///
/// Returns [`AssetFormat::Other`] for paths that don't match any
/// known structural marker. The scanner treats `Other` as "not an
/// asset" and excludes it from the result set.
pub fn classify_format(path: &Path) -> AssetFormat {
    // Fast path: extension-based classification for the four common
    // raster/vector formats. Case-insensitive.
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        match ext_lower.as_str() {
            "png" => return AssetFormat::Png,
            "jpg" | "jpeg" => return AssetFormat::Jpg,
            "webp" => return AssetFormat::WebP,
            "svg" => return AssetFormat::Svg,
            _ => {}
        }
    }

    // Slow path: structural markers for contextual metadata. The path
    // hint is enough to short-circuit some checks.
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // iOS `Contents.json` — must look like JSON to count as an asset
    // catalog. A file named `Contents.json` in a random subdirectory
    // is `Other`.
    if file_name == "Contents.json"
        || file_name.ends_with(".json") && file_name.starts_with("Contents")
    {
        let prefix = read_prefix(path);
        if looks_like_json(&prefix) {
            return AssetFormat::IosAssetCatalogJson;
        }
        return AssetFormat::Other("ios-contents-not-json".to_string());
    }

    // Android VectorDrawable / adaptive-icon — both XML, distinguished
    // by the first non-whitespace tag.
    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"))
    {
        let prefix = read_prefix(path);
        if looks_like_vector_drawable(&prefix) {
            return AssetFormat::AndroidVectorXml;
        }
        if looks_like_adaptive_icon(&prefix) {
            return AssetFormat::AndroidAdaptiveIconXml;
        }
    }

    // Runtime asset-manifest: filename `asset-manifest.js` (or
    // `asset-manifest-*.js`) plus the `__WORKMEN__` marker in the
    // file. The filename check is the fast path; the marker check is
    // the structural validation.
    if file_name == "asset-manifest.js"
        || (file_name.starts_with("asset-manifest") && file_name.ends_with(".js"))
    {
        let prefix = read_prefix(path);
        if looks_like_asset_manifest(&prefix) {
            return AssetFormat::RuntimeManifestJs;
        }
        // Filename says it *is* a manifest but the marker is missing;
        // we still classify it as the manifest format so the
        // validator can warn, but a caller that wants strict
        // structural validation can re-check.
        return AssetFormat::RuntimeManifestJs;
    }

    AssetFormat::Other(file_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fresh_tempdir(label: &str) -> std::path::PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("workmen-scan-fmt-{label}-{pid}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("create tempdir");
        dir
    }

    fn write(path: &std::path::Path, contents: &[u8]) {
        let mut f = std::fs::File::create(path).expect("create");
        f.write_all(contents).expect("write");
    }

    #[test]
    fn classifies_raster_extensions() {
        assert_eq!(classify_format(Path::new("a.png")), AssetFormat::Png);
        assert_eq!(classify_format(Path::new("a.jpg")), AssetFormat::Jpg);
        assert_eq!(classify_format(Path::new("a.jpeg")), AssetFormat::Jpg);
        assert_eq!(classify_format(Path::new("a.webp")), AssetFormat::WebP);
        assert_eq!(classify_format(Path::new("a.svg")), AssetFormat::Svg);
    }

    #[test]
    fn extension_check_is_case_insensitive() {
        assert_eq!(classify_format(Path::new("A.PNG")), AssetFormat::Png);
        assert_eq!(classify_format(Path::new("A.Svg")), AssetFormat::Svg);
    }

    #[test]
    fn contents_json_must_look_like_json() {
        let tmp = fresh_tempdir("contents-json");
        let path = tmp.join("Contents.json");
        write(&path, b"{ \"info\": { \"author\": \"xcode\" } }");
        assert_eq!(classify_format(&path), AssetFormat::IosAssetCatalogJson);

        // A file named Contents.json that is *not* JSON falls through
        // to Other (the design says: "a Contents.json in a random
        // subdirectory with no asset catalog structure is
        // UnsupportedFormat").
        let path2 = tmp.join("not-json.json");
        write(&path2, b"plain text");
        match classify_format(&path2) {
            AssetFormat::Other(_) => {}
            other => panic!("expected Other, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn android_vector_distinguished_from_adaptive_icon() {
        let tmp = fresh_tempdir("android");
        let vec_path = tmp.join("ic.xml");
        write(
            &vec_path,
            br#"<?xml version="1.0"?>
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="32dp"
    android:height="32dp">
</vector>"#,
        );
        assert_eq!(classify_format(&vec_path), AssetFormat::AndroidVectorXml);

        let adaptive_path = tmp.join("ic_adaptive.xml");
        write(
            &adaptive_path,
            br#"<?xml version="1.0"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
  <background android:drawable="@drawable/x"/>
</adaptive-icon>"#,
        );
        assert_eq!(
            classify_format(&adaptive_path),
            AssetFormat::AndroidAdaptiveIconXml
        );

        // Random XML is not AndroidVectorXml or AndroidAdaptiveIconXml.
        let random_xml = tmp.join("notes.xml");
        write(&random_xml, b"<?xml version=\"1.0\"?><note>hi</note>");
        assert_eq!(
            classify_format(&random_xml),
            AssetFormat::Other("notes.xml".to_string())
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn asset_manifest_requires_workmen_marker() {
        let tmp = fresh_tempdir("manifest");
        let path = tmp.join("asset-manifest.js");
        write(&path, br#"window.__WORKMEN__ = { "files": {} };"#);
        assert_eq!(classify_format(&path), AssetFormat::RuntimeManifestJs);

        // Filename says manifest but no marker.
        let path2 = tmp.join("asset-manifest.js");
        write(&path2, b"window.__SOMETHING_ELSE__ = function() {};");
        assert_eq!(classify_format(&path2), AssetFormat::RuntimeManifestJs);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn unknown_extension_returns_other() {
        let tmp = fresh_tempdir("unknown");
        let path = tmp.join("README.md");
        write(&path, b"# hello");
        match classify_format(&path) {
            AssetFormat::Other(_) => {}
            other => panic!("expected Other, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
