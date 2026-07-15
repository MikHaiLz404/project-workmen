//! Matcher primitives.
//!
//! Specificity is a deterministic *ordered tuple* of explicit
//! constraints. The tuple is documented here and returned by
//! [`ProfileResolver::specificity`] so callers (and reviewers)
//! can predict tie-breaking without reading the resolver.

use std::path::Path;

use crate::model::AssetRole;
use crate::model::ProfileMatcher;

/// One piece of a matcher's specificity. Higher is more specific.
/// The tuple ordering is part of the public contract -- do not
/// reorder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity {
    /// Number of extension filters (`.png`, `.svg`, ...). Higher
    /// is more specific.
    pub extensions: u8,
    /// Number of asset-role constraints. Higher is more specific.
    pub role: u8,
    /// Number of path-glob segments after stripping `**`. Counts
    /// `assets/ui/foo.png` as 3, `assets/ui/**` as 2.
    pub path_segments: u8,
    /// Whether the matcher has a naming token pattern.
    pub naming: bool,
}

impl Specificity {
    /// Build a `Specificity` for `matcher`. The function inspects
    /// each constraint and increments the appropriate counter.
    pub fn for_matcher(matcher: &ProfileMatcher) -> Self {
        let extensions = u8::from(matcher.extension.is_some());
        let role = u8::from(matcher.asset_role.is_some());
        let path_segments = matcher
            .path_glob
            .as_deref()
            .map(count_glob_segments)
            .unwrap_or(0);
        let naming = matcher.naming_pattern.is_some();
        Self {
            extensions,
            role,
            path_segments,
            naming,
        }
    }
}

/// Which kind of matcher matched an asset. Used by the resolver
/// for tie-breaking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatcherKind {
    /// No matcher satisfied -- the asset is unbound.
    None,
    /// The asset matched a profile's path glob.
    PathGlob,
    /// The asset matched an extension filter.
    Extension,
    /// The asset matched a naming token pattern.
    Naming,
}

/// Match `path` against a glob pattern. Supports `**` as a
/// directory wildcard.
pub fn match_path_glob(path: &str, glob: &str) -> bool {
    if glob == "**" {
        return true;
    }
    // Translate the glob into a regex-like check. We support:
    //   `prefix/**`     -> path starts with prefix/
    //   `prefix/*.ext`  -> top-level file in prefix/ with .ext
    //   `**/name`       -> any path ending in /name
    //   exact           -> equality
    if let Some(rest) = glob.strip_suffix("/**") {
        path.starts_with(rest) || path.starts_with(&format!("{rest}/")) || path == rest
    } else if let Some((prefix, suffix)) = glob.split_once("/*.") {
        // `prefix/*.ext` -> prefix/{name}.ext
        path.starts_with(&format!("{prefix}/"))
            && Path::new(path)
                .parent()
                .map(|p| p == Path::new(prefix))
                .unwrap_or(false)
            && std::path::Path::new(path)
                .extension()
                .is_some_and(|e| e.to_string_lossy() == suffix.trim_start_matches('.'))
    } else if let Some(name) = glob.strip_prefix("**/") {
        path == name || path.ends_with(&format!("/{name}"))
    } else {
        path == glob
    }
}

/// Match a stem against a token pattern like `btn-{{name}}`.
/// Returns `true` if the stem's structure matches the pattern.
pub fn match_token_pattern(stem: &str, pattern: &str) -> bool {
    // Very simple shape matcher: split both by `-`; require
    // every literal segment of the pattern to appear in the
    // stem in the same position; `{{...}}` segments are wild.
    let pat_parts: Vec<&str> = pattern.split('-').collect();
    let stem_parts: Vec<&str> = stem.split('-').collect();
    if pat_parts.len() != stem_parts.len() {
        return false;
    }
    pat_parts
        .iter()
        .zip(stem_parts.iter())
        .all(|(p, s)| p.starts_with("{{") || p == s)
}

fn count_glob_segments(glob: &str) -> u8 {
    glob.split('/')
        .filter(|s| !s.is_empty() && *s != "**")
        .count() as u8
}

/// True iff `role` satisfies the matcher's role constraint.
pub fn role_satisfies(role: AssetRole, matcher: &ProfileMatcher) -> bool {
    matcher.asset_role.is_none_or(|r| r == role)
}
