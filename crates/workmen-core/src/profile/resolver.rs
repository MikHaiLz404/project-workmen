//! Profile resolver.

use std::sync::Arc;

use thiserror::Error;

use crate::model::Asset;
use crate::model::Profile;
use crate::model::ProfileId;
use crate::model::ProfileMatcher;

use super::matcher::{Specificity, match_path_glob, match_token_pattern, role_satisfies};

/// Errors that `ProfileResolver::resolve` can return.
#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("ambiguous profile match: {amb:?}")]
    Ambiguous { amb: AmbiguousProfile },
    #[error("project config error: {0}")]
    Config(String),
}

/// The asset matched two or more profiles with the same
/// specificity. The error carries the candidate profiles so the
/// caller can prompt the user to disambiguate.
#[derive(Debug, Clone)]
pub struct AmbiguousProfile {
    pub candidates: Vec<Profile>,
    pub asset_path: String,
}

/// Resolves a single asset to its most-specific profile.
#[derive(Clone, Debug, Default)]
pub struct ProfileResolver {
    _private: Arc<()>,
}

impl ProfileResolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the specificity tuple for `matcher`. The tuple
    /// ordering is documented in [`super::matcher::Specificity`].
    pub fn specificity(&self, matcher: &ProfileMatcher) -> Specificity {
        Specificity::for_matcher(matcher)
    }

    /// Resolve `asset` against `profiles`. Returns:
    /// - `Ok(Some(&profile))` for a unique best match.
    /// - `Ok(None)` for no match.
    /// - `Err(ResolveError::Ambiguous)` for equal-specificity
    ///   matches.
    pub fn resolve<'a>(
        &self,
        asset: &Asset,
        profiles: &'a [Profile],
    ) -> Result<Option<&'a Profile>, ResolveError> {
        // 1. Filter to profiles that have at least one matching
        //    matcher.
        // 2. Among matches, score by the best matcher per profile.
        // 3. If exactly one best, return it.
        // 4. If two or more tie on specificity, return Ambiguous.
        struct Scored<'a> {
            profile: &'a Profile,
            spec: Specificity,
        }

        let mut best: Option<Scored<'a>> = None;
        let mut tied: Vec<&Profile> = Vec::new();

        for p in profiles {
            let Some(spec) = best_matcher_spec(asset, p) else {
                continue;
            };
            match &best {
                None => {
                    best = Some(Scored { profile: p, spec });
                    tied.clear();
                    tied.push(p);
                }
                Some(b) if spec > b.spec => {
                    best = Some(Scored { profile: p, spec });
                    tied.clear();
                    tied.push(p);
                }
                Some(b) if spec == b.spec => {
                    tied.push(p);
                }
                _ => {
                    // spec < best_spec; ignore.
                }
            }
        }

        if tied.len() > 1 {
            return Err(ResolveError::Ambiguous {
                amb: AmbiguousProfile {
                    candidates: tied.into_iter().cloned().collect(),
                    asset_path: asset.path.clone(),
                },
            });
        }
        Ok(best.map(|s| s.profile))
    }
}

/// Find the best-matching matcher on `profile` for `asset`, and
/// return its specificity. `None` if no matcher matches.
fn best_matcher_spec(asset: &Asset, profile: &Profile) -> Option<Specificity> {
    let mut best: Option<Specificity> = None;
    for m in &profile.matchers {
        if !matcher_matches(asset, m) {
            continue;
        }
        let spec = Specificity::for_matcher(m);
        best = Some(match best {
            None => spec,
            Some(b) if spec > b => spec,
            Some(b) => b,
        });
    }
    best
}

fn matcher_matches(asset: &Asset, matcher: &ProfileMatcher) -> bool {
    // Role filter.
    if !role_satisfies(asset.role, matcher) {
        return false;
    }
    // Path glob.
    if let Some(glob) = &matcher.path_glob
        && !match_path_glob(&asset.path, glob)
    {
        return false;
    }
    // Extension filter.
    if let Some(ext) = &matcher.extension {
        let file_ext = std::path::Path::new(&asset.path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if file_ext != ext {
            return false;
        }
    }
    // Naming pattern.
    if let Some(pattern) = &matcher.naming_pattern {
        let stem = std::path::Path::new(&asset.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if !match_token_pattern(stem, pattern) {
            return false;
        }
    }
    true
}

// Local helper to anchor ProfileId import.
#[allow(dead_code)]
fn _pid_anchor(_id: &ProfileId) {}
