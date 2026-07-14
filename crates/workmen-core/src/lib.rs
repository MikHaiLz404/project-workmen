//! Workmen core library: domain models, error type, and shared traits.
//!
//! This crate is the single source of truth for the Workmen type system.
//! The CLI and any future host binaries depend on it; nothing in here
//! depends on the CLI. Domain types (`Asset`, `Profile`,
//! `ValidationIssue`, ...) live in [`workmen_core::model`].

pub mod classify;
pub mod error;
pub mod model;
pub mod project;
pub mod scan;
pub mod schema;

pub use error::WorkmenError;
