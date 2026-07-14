//! Asset-role classification.

use std::collections::HashSet;

use crate::model::AssetFormat;
use crate::model::AssetRole;
use crate::project::ProjectConfig;
use crate::scan::ScanResult;
use crate::scan::ScannedFile;

/// The threshold-based policy that drives classification. All
/// scoring constants live here, not scattered through the code.
#[derive(Clone, Debug)]
pub struct ClassificationPolicy {
    /// Score at or above this threshold is a high-confidence
    /// assignment.
    pub high_confidence_threshold: i32,
    /// Score at or above this threshold is a low-confidence
    /// assignment. Below this is `Unclassified`.
    pub low_confidence_threshold: i32,
    /// Project-relative path prefixes that imply `Runtime`.
    pub runtime_path_prefixes: Vec<String>,
    /// Project-relative path prefixes that imply `MirrorTarget`
    /// (per design §12 Mirror Targets). Each mirror target is
    /// linked back to a single runtime asset, not a new
    /// inventory root.
    pub mirror_path_prefixes: Vec<String>,
    /// Project-relative path prefixes that imply `Source`.
    pub source_path_prefixes: Vec<String>,
    /// Path components that imply `Excluded` when any of these
    /// tokens appear in the project-relative path.
    pub excluded_names: Vec<String>,
    /// File extensions that imply `Excluded`.
    pub excluded_extensions: Vec<String>,
    /// Formats that count as contextual metadata (iOS / Android /
    /// runtime manifest). These carry `EvidenceReason::ContextualMetadata`
    /// to boost their score.
    pub contextual_metadata_formats: HashSet<AssetFormat>,
}

/// One piece of evidence collected during classification. The
/// `reason` is named so the user (and tests) can see *why* a role
/// was assigned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub reason: EvidenceReason,
    /// Score delta. The final confidence is the sum of weights.
    pub weight: i32,
}

/// Named reasons the classifier records. Adding a new variant is a
/// public API change — downstream tests and the design doc
/// (`docs/superpowers/specs/2026-07-13-workmen-design.md` §6)
/// reference them by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EvidenceReason {
    /// Path starts with a `source_path_prefixes` entry.
    SourcePathPrefix,
    /// Path starts with a `runtime_path_prefixes` entry.
    RuntimePathPrefix,
    /// Path starts with a `mirror_path_prefixes` entry.
    MirrorPathPrefix,
    /// Path contains an `excluded_names` token.
    ExcludedName,
    /// Path has an `excluded_extensions` extension.
    ExcludedExtension,
    /// Format is a contextual metadata format
    /// (`IosAssetCatalogJson`, `AndroidVectorXml`, etc.).
    ContextualMetadata,
    /// Hash matches a known runtime asset hash.
    HashMatchesKnownRuntime,
    /// No positive evidence was found. The classifier refused to
    /// guess.
    NoPositiveEvidence,
}

/// The accumulated evidence score for an asset. The `score` is the
/// sum of evidence weights; `reasons` is the list of named
/// reasons that produced it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Confidence {
    pub score: i32,
    pub reasons: Vec<EvidenceReason>,
}

/// One classified asset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleAssignment {
    pub asset_path: String,
    pub role: AssetRole,
    pub confidence: Confidence,
    /// For `MirrorTarget` assignments, the path of the runtime
    /// asset this mirror is linked to. The plan says "Assert
    /// generated web/iOS/Android copies are linked as Mirror
    /// Targets to one Runtime asset rather than emitted as
    /// independent inventory roots."
    pub mirror_target_of: Option<String>,
}

/// An asset that did not classify with sufficient confidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnclassifiedEntry {
    pub asset_path: String,
    /// The strongest reason the classifier saw (lowest weight at
    /// zero or negative score).
    pub why: EvidenceReason,
    pub observed_evidence: Vec<Evidence>,
}

/// Counts of assignments by role. Surfaced in the result so
/// downstream tools can show a summary without re-counting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PolicySummary {
    pub total: usize,
    pub source_count: usize,
    pub runtime_count: usize,
    pub derived_count: usize,
    pub mirror_target_count: usize,
    pub excluded_count: usize,
    pub unclassified_count: usize,
}

/// The full output of the classifier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClassificationResult {
    pub assignments: Vec<RoleAssignment>,
    pub unclassified: Vec<UnclassifiedEntry>,
    pub policy_summary: PolicySummary,
}

/// Classify every file in `scan` according to `policy`. The
/// optional `config` is currently unused (T5 does not yet bind
/// paths from `.workmen/project.yaml`); future tasks may consult
/// it.
pub fn classify(
    scan: &ScanResult,
    _config: Option<&ProjectConfig>,
    policy: &ClassificationPolicy,
) -> ClassificationResult {
    // Build a set of runtime hashes so mirror targets can link back
    // to their runtime asset (when a hash match is available).
    let runtime_hashes: std::collections::HashMap<&str, &str> = scan
        .files
        .iter()
        .filter(|f| starts_with_any(&f.path, &policy.runtime_path_prefixes))
        .filter_map(|f| {
            f.blake3_hash
                .as_ref()
                .map(|h| (h.as_str(), f.path.as_str()))
        })
        .collect();

    let mut assignments = Vec::new();
    let mut unclassified = Vec::new();

    for file in &scan.files {
        let (role, confidence) = classify_one(file, policy, &runtime_hashes);
        match role {
            AssetRole::Unclassified => {
                unclassified.push(UnclassifiedEntry {
                    asset_path: file.path.clone(),
                    why: confidence
                        .reasons
                        .first()
                        .copied()
                        .unwrap_or(EvidenceReason::NoPositiveEvidence),
                    observed_evidence: vec![Evidence {
                        reason: EvidenceReason::NoPositiveEvidence,
                        weight: 0,
                    }],
                });
            }
            _ => {
                let mirror_target_of = if matches!(role, AssetRole::MirrorTarget) {
                    file.blake3_hash
                        .as_ref()
                        .and_then(|h| runtime_hashes.get(h.as_str()).copied().map(String::from))
                } else {
                    None
                };
                assignments.push(RoleAssignment {
                    asset_path: file.path.clone(),
                    role,
                    confidence,
                    mirror_target_of,
                });
            }
        }
    }

    // Sort by path for deterministic output.
    assignments.sort_by(|a, b| a.asset_path.cmp(&b.asset_path));
    unclassified.sort_by(|a, b| a.asset_path.cmp(&b.asset_path));

    // Summary counts.
    let mut policy_summary = PolicySummary {
        total: assignments.len() + unclassified.len(),
        ..Default::default()
    };
    for a in &assignments {
        match a.role {
            AssetRole::Source => policy_summary.source_count += 1,
            AssetRole::Runtime => policy_summary.runtime_count += 1,
            AssetRole::Derived => policy_summary.derived_count += 1,
            AssetRole::MirrorTarget => policy_summary.mirror_target_count += 1,
            AssetRole::Excluded => policy_summary.excluded_count += 1,
            AssetRole::Unclassified => policy_summary.unclassified_count += 1,
        }
    }
    policy_summary.unclassified_count += unclassified.len();

    ClassificationResult {
        assignments,
        unclassified,
        policy_summary,
    }
}

/// Classify a single file. Returns `(role, confidence)`. Exposed
/// for unit tests; the public entry point is [`classify`].
fn classify_one(
    file: &ScannedFile,
    policy: &ClassificationPolicy,
    runtime_hashes: &std::collections::HashMap<&str, &str>,
) -> (AssetRole, Confidence) {
    let mut evidence = Vec::new();

    // 1. Excluded evidence (highest priority).
    for tok in &policy.excluded_names {
        if path_contains_token(&file.path, tok) {
            evidence.push(Evidence {
                reason: EvidenceReason::ExcludedName,
                weight: 100,
            });
        }
    }
    for ext in &policy.excluded_extensions {
        if file.path.ends_with(ext) {
            evidence.push(Evidence {
                reason: EvidenceReason::ExcludedExtension,
                weight: 100,
            });
        }
    }

    // 2. Mirror-target evidence (decisive — design §12 says
    // web/iOS/Android copies are *always* MirrorTarget, not
    // independent inventory roots).
    for prefix in &policy.mirror_path_prefixes {
        if starts_with(&file.path, prefix) {
            evidence.push(Evidence {
                reason: EvidenceReason::MirrorPathPrefix,
                weight: 80,
            });
            break;
        }
    }

    // 3. Runtime evidence.
    if starts_with_any(&file.path, &policy.runtime_path_prefixes) {
        evidence.push(Evidence {
            reason: EvidenceReason::RuntimePathPrefix,
            weight: 50,
        });
    }

    // 4. Source evidence.
    if starts_with_any(&file.path, &policy.source_path_prefixes) {
        evidence.push(Evidence {
            reason: EvidenceReason::SourcePathPrefix,
            weight: 40,
        });
    }

    // 5. Contextual-metadata evidence.
    if policy.contextual_metadata_formats.contains(&file.format) {
        evidence.push(Evidence {
            reason: EvidenceReason::ContextualMetadata,
            weight: 10,
        });
    }

    // 6. Hash match (mirror target linking).
    if let Some(hash) = &file.blake3_hash
        && runtime_hashes.contains_key(hash.as_str())
    {
        evidence.push(Evidence {
            reason: EvidenceReason::HashMatchesKnownRuntime,
            weight: 60,
        });
    }

    let score: i32 = evidence.iter().map(|e| e.weight).sum();
    let reasons: Vec<EvidenceReason> = evidence.iter().map(|e| e.reason).collect();

    // Decision: excluded always wins. Mirror-target has high weight
    // (80) so a mirror-prefixed file is almost always MirrorTarget.
    // Runtime (50) beats Source (40). Contextual metadata boosts
    // without flipping the decision.
    let role = if reasons.contains(&EvidenceReason::ExcludedName)
        || reasons.contains(&EvidenceReason::ExcludedExtension)
    {
        AssetRole::Excluded
    } else if reasons.contains(&EvidenceReason::MirrorPathPrefix) {
        AssetRole::MirrorTarget
    } else if reasons.contains(&EvidenceReason::RuntimePathPrefix) {
        AssetRole::Runtime
    } else if reasons.contains(&EvidenceReason::SourcePathPrefix) {
        AssetRole::Source
    } else if score >= policy.low_confidence_threshold {
        // Some positive evidence but no path-prefix match. Default
        // to Unclassified at low confidence; the caller can resolve.
        AssetRole::Unclassified
    } else {
        AssetRole::Unclassified
    };

    let confidence = Confidence { score, reasons };
    (role, confidence)
}

fn starts_with(path: &str, prefix: &str) -> bool {
    path.starts_with(prefix) || path == prefix.trim_end_matches('/')
}

fn starts_with_any(path: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|p| starts_with(path, p))
}

fn path_contains_token(path: &str, token: &str) -> bool {
    path.split(['/', '\\']).any(|c| {
        // Match if the component equals the token directly (e.g.
        // `/deprecated/`), or the component's stem matches (e.g.
        // `old.svg` -> stem `old`).
        if c == token {
            return true;
        }
        std::path::Path::new(c)
            .file_stem()
            .map(|s| s == token)
            .unwrap_or(false)
    })
}
