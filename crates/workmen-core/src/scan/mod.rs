//! Project scanner.
//!
//! `scan_project` walks a [`ProjectRoot`] and produces a [`ScanResult`]
//! of [`ScannedFile`]s plus [`ScanDiagnostic`]s for any per-file
//! decode / IO failures. The scanner is read-only: it does not
//! modify the inspected project, does not follow symlinks, and
//! continues after per-file errors.
//!
//! See the design's §4 Project Scanner for the full contract. The
//! scanner never silently fails: every problem surfaces as a
//! diagnostic so the validator and the user can act on it.

mod cache;
mod formats;
mod metadata;
mod walker;

pub use cache::{CacheEntry, DecodedMeta, ScanCache, blake3_hex};
pub use metadata::{RasterMeta, VectorMeta, decode_raster_metadata, decode_vector_metadata};
pub use walker::{
    DiagnosticKind, ScanDiagnostic, ScanMode, ScanRequest, ScanResult, ScannedFile, scan_project,
};
