//! Profile resolution and lifecycle.
//!
//! `ProfileResolver::resolve` returns the most-specific profile
//! that matches an asset, or `AmbiguousProfile` if two or more
//! profiles tie on specificity. Specificity is a deterministic
//! ordered tuple (see [`ProfileResolver::specificity`]) -- not a
//! floating-point score -- so the resolver behaves identically
//! across runs and platforms.
//!
//! `ProfileLifecycle` is a small in-memory CRUD over a project's
//! profile set: draft edits, active validation, locked rejection,
//! unlock with reason, and revision increment on every state
//! transition.
//!
//! Both modules are read-only with respect to the inspected
//! project on disk. The lifecycle writes to its own in-memory
//! state and is a stage for the next CLI command.

mod lifecycle;
mod matcher;
mod resolver;

pub use lifecycle::{LifecycleError, ProfileLifecycle};
pub use matcher::{MatcherKind, Specificity, match_path_glob, match_token_pattern};
pub use resolver::{AmbiguousProfile, ProfileResolver, ResolveError};

/// Re-export the common error-type alias used by `resolve`.
pub type ResolveResult<T> = Result<T, ResolveError>;
