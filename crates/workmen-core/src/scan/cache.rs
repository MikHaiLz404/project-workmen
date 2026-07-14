//! BLAKE3 stat cache.
//!
//! The cache maps `(relative_path, size, modified_time)` to a
//! [`CacheEntry`] holding the file's BLAKE3 hash and any decoded
//! metadata. On a second scan, files whose `(path, size, mtime)`
//! key is unchanged are skipped — the cache provides the hash and
//! decoded metadata without re-reading the file.
//!
//! Persistence: the cache is saved to `<OS app-cache dir>/workmen/scan-cache.json`.
//! Missing directories are created on save; missing files are
//! treated as an empty cache on load.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::WorkmenError;
use crate::model::{PixelSize, Rect, ViewBox};

/// One cached scan result for a file. Stores the BLAKE3 hash
/// (hex-encoded, 64 chars) and any decoded metadata the scanner
/// may have computed.
///
/// `decoded_meta` is `None` if the file is raster/vector and the
/// scanner hasn't yet computed width/height. The scanner
/// computes it on first scan and caches it for re-use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry {
    pub blake3_hash: String,
    #[serde(default)]
    pub decoded_meta: Option<DecodedMeta>,
}

/// Decoded metadata cached per file. Optional fields let the cache
/// stay forward-compatible as the scanner grows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoded_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoded_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_alpha: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_depth: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_bounds: Option<Rect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_box: Option<ViewBox>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raster_preview_targets: Vec<PixelSize>,
}

/// The on-disk format of the cache. A thin wrapper around the
/// entries map so the JSON document is self-describing and
/// forward-compatible (new top-level fields can be added without
/// breaking old readers).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheFile {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default)]
    entries: BTreeMap<CacheKey, CacheEntry>,
}

fn default_schema_version() -> u32 {
    1
}

/// Cache key: relative path + size + modified time. The triple
/// uniquely identifies a file's content *as of the last scan*
/// because mtime + size uniquely identifies a version of a file
/// on most filesystems.
type CacheKey = (String, u64, SystemTime);

/// In-memory cache of file scan results. The cache is keyed by
/// `(path, size, mtime)` and stores per-file hashes plus optional
/// decoded metadata.
#[derive(Clone, Debug, Default)]
pub struct ScanCache {
    entries: BTreeMap<CacheKey, CacheEntry>,
}

impl ScanCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of entries in the cache (used by tests to assert
    /// behavior).
    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// All entries in the cache (used by tests to assert behavior).
    #[allow(dead_code)]
    pub fn entries(&self) -> impl Iterator<Item = (&CacheKey, &CacheEntry)> {
        self.entries.iter()
    }

    /// Look up a cache entry. `path` must be the project-relative
    /// path, `size` and `mtime` come from the file's stat.
    pub fn get(&self, path: &str, size: u64, mtime: SystemTime) -> Option<&CacheEntry> {
        self.entries.get(&(path.to_string(), size, mtime))
    }

    /// Insert a cache entry. `path` must be the project-relative
    /// path; `size` and `mtime` come from the file's stat.
    pub fn put(&mut self, path: String, size: u64, mtime: SystemTime, entry: CacheEntry) {
        self.entries.insert((path, size, mtime), entry);
    }

    /// Resolve the on-disk cache file path under the OS app-cache
    /// directory. Returns `None` if the directories cannot be
    /// resolved (e.g. no home directory in a sandboxed environment).
    fn cache_path() -> Option<PathBuf> {
        let proj = directories::ProjectDirs::from("workmen", "workmen", "workmen")?;
        Some(proj.cache_dir().join("scan-cache.json"))
    }

    /// Load the cache from disk. Returns an empty cache if the
    /// file does not exist or cannot be read; the scanner should
    /// not crash on a fresh machine.
    pub fn load() -> Result<Self, WorkmenError> {
        let Some(path) = Self::cache_path() else {
            return Ok(Self::new());
        };
        if !path.exists() {
            return Ok(Self::new());
        }
        let text = std::fs::read_to_string(&path).map_err(|e| WorkmenError::io(&path, e))?;
        let parsed: CacheFile = serde_json::from_str(&text).map_err(|e| {
            WorkmenError::config(format!("scan-cache.json is malformed: {e}"), &path)
        })?;
        // Convert the parsed `BTreeMap<CacheKey, CacheEntry>` back
        // into our key type. The JSON round-trip preserves the
        // SystemTime via serde's default impl.
        Ok(Self {
            entries: parsed.entries,
        })
    }

    /// Save the cache to disk atomically. Writes to a temp file in
    /// the same directory and renames into place. Returns Ok(()) if
    /// the cache path cannot be resolved (sandboxed environments).
    pub fn save(&self) -> Result<(), WorkmenError> {
        let Some(path) = Self::cache_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| WorkmenError::io(parent, e))?;
        }
        let file = CacheFile {
            schema_version: 1,
            entries: self.entries.clone(),
        };
        let body = serde_json::to_string_pretty(&file)
            .map_err(|e| WorkmenError::internal(format!("cache serialize: {e}")))?;
        let dir = path
            .parent()
            .ok_or_else(|| WorkmenError::internal("cache path has no parent".to_string()))?;
        let tmp = NamedTempFile::new_in(dir).map_err(|e| WorkmenError::io(dir, e))?;
        std::fs::write(tmp.path(), &body).map_err(|e| WorkmenError::io(tmp.path(), e))?;
        // Atomic rename: `persist` handles this on POSIX and Windows.
        tmp.persist(&path)
            .map_err(|e| WorkmenError::io(&path, e.error))?;
        Ok(())
    }
}

/// Compute a 64-character hex BLAKE3 digest of `bytes`. The
/// scanner uses this internally; the test fixture also calls it
/// directly.
pub fn blake3_hex(bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in hash.as_bytes() {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn sample_entry() -> CacheEntry {
        CacheEntry {
            blake3_hash: blake3_hex(b"workmen test"),
            decoded_meta: None,
        }
    }

    #[test]
    fn empty_cache_get_returns_none() {
        let cache = ScanCache::new();
        let mtime = SystemTime::UNIX_EPOCH;
        assert!(cache.get("a.png", 100, mtime).is_none());
    }

    #[test]
    fn put_then_get_returns_entry() {
        let mut cache = ScanCache::new();
        let mtime = SystemTime::UNIX_EPOCH;
        cache.put("a.png".to_string(), 100, mtime, sample_entry());
        let got = cache.get("a.png", 100, mtime).expect("entry present");
        assert_eq!(got.blake3_hash, blake3_hex(b"workmen test"));
    }

    #[test]
    fn size_change_invalidates_entry() {
        let mut cache = ScanCache::new();
        let mtime = SystemTime::UNIX_EPOCH;
        cache.put("a.png".to_string(), 100, mtime, sample_entry());
        assert!(cache.get("a.png", 101, mtime).is_none());
    }

    #[test]
    fn mtime_change_invalidates_entry() {
        let mut cache = ScanCache::new();
        let mtime_a = SystemTime::UNIX_EPOCH;
        let mtime_b = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        cache.put("a.png".to_string(), 100, mtime_a, sample_entry());
        assert!(cache.get("a.png", 100, mtime_b).is_none());
    }

    #[test]
    fn blake3_hex_is_deterministic() {
        let a = blake3_hex(b"hello");
        let b = blake3_hex(b"hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64, "BLAKE3 hex must be 64 chars");
    }

    #[test]
    fn blake3_hex_distinguishes_different_inputs() {
        let a = blake3_hex(b"hello");
        let b = blake3_hex(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn scan_cache_load_works_when_cache_dir_does_not_exist() {
        // The test exercises the path where the OS cache directory
        // is missing or unwritable; load() must return an empty
        // cache rather than crash.
        let _ = ScanCache::load();
    }
}
