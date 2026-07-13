//! Workmen core library: domain models, error type, and shared traits.
//!
//! This crate is the single source of truth for the Workmen type system. The
//! CLI and any future host binaries depend on it; nothing in here depends on
//! the CLI. Domain types (asset, source, runtime, derived, mirror target,
//! excluded, unclassified) land in Task 2.

pub mod error;

pub use error::WorkmenError;
