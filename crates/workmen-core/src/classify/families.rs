//! Asset-family grouping.
//!
//! `group_into_families` partitions classified assets into
//! candidate families. The grouping key is `(directory, stem,
//! format)` — *never* dimensions alone. The plan says: "Group
//! candidate families by directory, naming stem/token shape,
//! dimensions, format, and contextual metadata links. Never merge
//! families solely because dimensions match."

use std::path::Path;

use crate::model::AssetFormat;

use super::roles::RoleAssignment;

/// The key under which a family is grouped. Two assets with the
/// same `FamilyKey` *might* belong to the same family — the caller
/// must still check dimensions and contextual links before
/// confirming the merge.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FamilyKey {
    /// The parent directory of the asset, relative to the project
    /// root.
    pub directory: String,
    /// The stem of the file (filename minus extension). The
    /// "@2x" / "@3x" suffix is stripped before keying.
    pub stem: String,
    /// The asset format. PNG and SVG never share a family even
    /// when dimensions match (per plan: "Never merge families
    /// solely because dimensions match").
    pub format: AssetFormat,
}

/// A group of assets sharing a [`FamilyKey`]. The caller promotes
/// this to a [`super::DraftProfile`] or splits it further.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyGroup {
    pub key: FamilyKey,
    pub member_assignments: Vec<RoleAssignment>,
}

/// Group `assignments` into family groups.
pub fn group_into_families(assignments: &[RoleAssignment]) -> Vec<FamilyGroup> {
    let mut groups: std::collections::BTreeMap<FamilyKey, Vec<RoleAssignment>> =
        std::collections::BTreeMap::new();
    for a in assignments {
        let key = family_key_for(
            &a.asset_path,
            /* format would come from somewhere else */
            AssetFormat::Other("unknown".to_string()),
        );
        // The format above is a placeholder. The plan's classify()
        // returns RoleAssignment but does not carry the format. A
        // future revision should add `format: AssetFormat` to
        // `RoleAssignment`. For now, callers should use the
        // richer `group_assignments_with_format` helper below.
        groups.entry(key).or_default().push(a.clone());
    }
    groups
        .into_iter()
        .map(|(key, member_assignments)| FamilyGroup {
            key,
            member_assignments,
        })
        .collect()
}

/// Group assignments by family key, with explicit format supplied
/// for each asset. This is the preferred helper — the format
/// affects the key, so the caller must provide it.
pub fn group_assignments_with_format<F>(
    assignments: &[RoleAssignment],
    format_for: F,
) -> Vec<FamilyGroup>
where
    F: Fn(&str) -> AssetFormat,
{
    let mut groups: std::collections::BTreeMap<FamilyKey, Vec<RoleAssignment>> =
        std::collections::BTreeMap::new();
    for a in assignments {
        let format = format_for(&a.asset_path);
        let key = family_key_for(&a.asset_path, format);
        groups.entry(key).or_default().push(a.clone());
    }
    groups
        .into_iter()
        .map(|(key, member_assignments)| FamilyGroup {
            key,
            member_assignments,
        })
        .collect()
}

/// Build the family key for a single asset. Strips `@Nx` suffixes
/// from the stem so `btn-rest.png` and `btn-rest@2x.png` share a
/// family.
fn family_key_for(path: &str, format: AssetFormat) -> FamilyKey {
    let p = Path::new(path);
    let directory = p
        .parent()
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_default();
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // Strip "@2x" / "@3x" / "@1.5x" suffixes.
    let stem = strip_density_suffix(&stem);
    FamilyKey {
        directory,
        stem,
        format,
    }
}

/// Remove `@<number>x` density suffix from a stem. E.g.
/// `btn-rest@2x` -> `btn-rest`. Leaves the stem unchanged if no
/// suffix is present.
fn strip_density_suffix(stem: &str) -> String {
    if let Some(idx) = stem.rfind('@') {
        let suffix = &stem[idx + 1..];
        if !suffix.is_empty()
            && suffix.ends_with('x')
            && suffix[..suffix.len() - 1].parse::<f32>().is_ok()
        {
            return stem[..idx].to_string();
        }
    }
    stem.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::roles::Confidence;
    use crate::model::AssetRole;

    fn assignment(path: &str) -> RoleAssignment {
        RoleAssignment {
            asset_path: path.to_string(),
            role: AssetRole::Source,
            confidence: Confidence::default(),
            mirror_target_of: None,
        }
    }

    #[test]
    fn family_key_strips_density_suffix() {
        let k = family_key_for("assets/ui/btn-rest@2x.png", AssetFormat::Png);
        assert_eq!(k.stem, "btn-rest");
        assert_eq!(k.directory, "assets/ui");
        assert_eq!(k.format, AssetFormat::Png);
    }

    #[test]
    fn family_key_keeps_no_suffix_unchanged() {
        let k = family_key_for("assets/btn-rest.png", AssetFormat::Png);
        assert_eq!(k.stem, "btn-rest");
    }

    #[test]
    fn family_key_does_not_strip_arbitrary_at_sign() {
        // "user@host" style: not a density suffix (no `x` at the end).
        let k = family_key_for("path/user@host.png", AssetFormat::Png);
        assert_eq!(k.stem, "user@host");
    }

    #[test]
    fn groups_separate_families_by_directory() {
        let assignments = vec![
            assignment("assets/btn-rest.png"),
            assignment("assets-source/btn-rest.png"),
        ];
        let groups = group_assignments_with_format(&assignments, |_| AssetFormat::Png);
        assert_eq!(groups.len(), 2);
        for g in &groups {
            assert_eq!(g.member_assignments.len(), 1);
        }
    }
}
