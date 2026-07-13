# Workmen Core, Scanner, Profiles, and Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the Project Scanner as a native `workmen` CLI and reusable Rust core that can scan a game project read-only, infer asset roles and Draft Profiles, resolve locked contracts, validate assets, and emit text, JSON, and SARIF reports.

**Architecture:** A Cargo workspace contains `workmen-core` and a thin `workmen-cli`. The core exposes typed application services and serializable contracts; filesystem, image-decoding, and clock effects sit behind adapters so tests use temporary synthetic projects. This plan deliberately excludes GUI and asset mutation.

**Tech Stack:** Rust 1.95 / edition 2024, Cargo workspace, `clap`, `serde`, `serde_yaml`, `serde_json`, `schemars`, `thiserror`, `ignore`, `globset`, `image`, `blake3`, `tracing`, `tracing-appender`, `directories`, `rayon`, `tempfile`, `assert_cmd`, `predicates`, `insta`.

## Global Constraints

- The design source of truth is `docs/superpowers/specs/2026-07-13-workmen-design.md`.
- Scanning must not modify the inspected project. Only `workmen init` may create `.workmen/`, after explicit `--confirm`.
- Do not follow symlinks. Continue after per-file decode/read errors and report each failure.
- Use project-relative paths in models, reports, and logs. Never serialize image bytes into logs.
- Source, Runtime, Derived, Mirror Target, Excluded, and Unclassified remain distinct roles.
- Every new rule starts with a failing test. Run focused tests before the workspace suite.
- Each task ends with a small commit; do not combine plan tasks.

## File Structure Map

- `crates/workmen-core/src/model/`: serialized domain vocabulary shared by all frontends.
- `crates/workmen-core/src/project/`: root discovery and versioned `.workmen` contract persistence.
- `crates/workmen-core/src/scan/` and `src/classify/`: read-only discovery, metadata, roles, families, and drafts.
- `crates/workmen-core/src/profile/` and `src/validate/`: deterministic resolution and rule evaluation.
- `crates/workmen-core/src/report/`: text, JSON, and SARIF projections of domain results.
- `crates/workmen-core/src/service.rs`: the only frontend-facing orchestration API.
- `crates/workmen-cli/`: argument parsing, output selection, and process exit mapping only.

---

### Task 1: Bootstrap the Rust workspace and executable seam

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `crates/workmen-core/Cargo.toml`
- Create: `crates/workmen-core/src/lib.rs`
- Create: `crates/workmen-core/src/error.rs`
- Create: `crates/workmen-cli/Cargo.toml`
- Create: `crates/workmen-cli/src/main.rs`
- Create: `crates/workmen-cli/tests/help.rs`

**Interfaces produced:** `workmen_core::WorkmenError`; native `workmen` binary.

- [ ] Write `crates/workmen-cli/tests/help.rs` first:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn help_names_the_read_only_scan_command() {
    Command::cargo_bin("workmen").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("scan"))
        .stdout(contains("validate"))
        .stdout(contains("init"));
}
```

- [ ] Run `rtk cargo test -p workmen-cli --test help`; expect failure because the workspace and binary do not exist.
- [ ] Create a workspace with resolver `2`, edition `2024`, and `rust-version = "1.95"`. Pin the Rust channel to `1.95.0`.
- [ ] Add a `clap` parser with `scan`, `validate`, and `init` subcommands. Each may return a clear `command is not implemented` error until its task lands; `--help` must exit successfully.
- [ ] Define `WorkmenError::{Config, Io, Decode, Validation, Internal}` with path-aware messages and no absolute-path serialization helper.
- [ ] Run `rtk cargo fmt --check && rtk cargo clippy --workspace --all-targets -- -D warnings && rtk cargo test --workspace`; expect all green.
- [ ] Commit: `rtk git add Cargo.toml rust-toolchain.toml .gitignore crates/workmen-core crates/workmen-cli && rtk git commit -m "build: bootstrap Workmen Rust workspace"`.

### Task 2: Define canonical domain contracts and generated schemas

**Files:**
- Create: `crates/workmen-core/src/model/mod.rs`
- Create: `crates/workmen-core/src/model/asset.rs`
- Create: `crates/workmen-core/src/model/profile.rs`
- Create: `crates/workmen-core/src/model/validation.rs`
- Create: `crates/workmen-core/src/model/operation.rs`
- Create: `crates/workmen-core/src/schema.rs`
- Create: `crates/workmen-core/tests/contracts.rs`
- Create: `schemas/workmen-project.schema.json`
- Create: `schemas/workmen-profile.schema.json`

**Interfaces produced:** `Asset`, `AssetRole`, `AssetFormat`, `AssetFamily`, `Profile`, `ProfileState`, `PlatformBudget`, `ValidationIssue`, `Severity`, `SpecDiff`, `OperationEvent`.

```rust
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Asset { pub id: AssetId, pub path: String, pub role: AssetRole, pub format: AssetFormat, pub metadata: AssetMetadata }

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Profile { pub schema_version: u32, pub id: ProfileId, pub profile_revision: u32, pub state: ProfileState, pub matchers: Vec<ProfileMatcher>, pub budgets: Vec<PlatformBudget> }
```

- [ ] Write serialization tests that assert stable values such as `mirrorTarget`, `unclassified`, and `profileRevision`, plus rejection of an unknown `schemaVersion`.
- [ ] Define IDs as transparent newtypes (`AssetId`, `ProfileId`, `FamilyId`) and store paths as normalized project-relative UTF-8 strings. Do not leak `PathBuf` through serialized contracts.
- [ ] Model raster metadata (`width`, `height`, `encodedBytes`, `decodedBytes`, `hasAlpha`, `colorType`, `bitDepth`, optional `alphaBounds`) and vector metadata (`viewBox`, raster preview targets).
- [ ] Model Profile matchers, naming rules, source/runtime relationships, exceptions with reason and optional expiry, platform budgets, and lifecycle `Draft | Active | Locked`.
- [ ] Make `SpecDiff` carry `ruleId`, `profileId`, `expected`, `actual`, `platform`, `severity`, and `suggestedAction`.
- [ ] Generate the two checked-in JSON Schemas from Rust types in the contract test and fail when generated output differs from the files. This makes Rust the schema source of truth.
- [ ] Run `rtk cargo test -p workmen-core --test contracts`; expect stable snapshots and schemas.
- [ ] Commit: `rtk git add crates/workmen-core schemas && rtk git commit -m "feat(core): define Workmen domain contracts"`.

### Task 3: Load and initialize project-local contracts safely

**Files:**
- Create: `crates/workmen-core/src/project/mod.rs`
- Create: `crates/workmen-core/src/project/root.rs`
- Create: `crates/workmen-core/src/project/config.rs`
- Create: `crates/workmen-core/src/project/init.rs`
- Create: `crates/workmen-core/src/project/migrate.rs`
- Create: `crates/workmen-core/tests/project_contract.rs`
- Create: `crates/workmen-core/tests/fixtures/projects/empty-game/.gitignore`

**Interfaces consumed:** generated project/profile schemas.
**Interfaces produced:** `ProjectRoot::discover`, `ProjectConfig::load`, `ProjectInitializer::preview`, `ProjectInitializer::commit`.

```rust
impl ProjectRoot { pub fn discover(start: &Path) -> Result<Self, WorkmenError>; }
impl ProjectConfig { pub fn load(root: &ProjectRoot) -> Result<Option<Self>, WorkmenError>; }
impl ProjectInitializer {
    pub fn preview(root: &ProjectRoot) -> Result<InitPreview, WorkmenError>;
    pub fn commit(preview: InitPreview, confirmed: bool) -> Result<ProjectConfig, WorkmenError>;
}
```

- [ ] Write tests for root discovery, absent `.workmen` configuration, invalid YAML, supported schema migration with revision preservation, unsupported schema version, and refusal to initialize without confirmation.
- [ ] Test that `preview()` returns the exact proposed paths and bytes but leaves the fixture unchanged.
- [ ] Implement upward root discovery using `.git` or `.workmen`; a supplied directory remains the root when neither marker exists.
- [ ] Load `.workmen/project.yaml` and `.workmen/specs/*.yaml`, validate against the typed contracts, and surface line/path context on parse failures.
- [ ] Implement atomic initialization through a sibling staging directory and rename. Generate only `project.yaml` and an empty `specs/` directory.
- [ ] Wire `workmen init <path>` to print the preview; require `--confirm` to write.
- [ ] Run `rtk cargo test -p workmen-core --test project_contract && rtk cargo test -p workmen-cli`.
- [ ] Commit: `rtk git add crates/workmen-core crates/workmen-cli && rtk git commit -m "feat(core): add safe project initialization"`.

### Task 4: Scan supported art and contextual metadata

**Files:**
- Create: `crates/workmen-core/src/scan/mod.rs`
- Create: `crates/workmen-core/src/scan/walker.rs`
- Create: `crates/workmen-core/src/scan/formats.rs`
- Create: `crates/workmen-core/src/scan/metadata.rs`
- Create: `crates/workmen-core/src/scan/cache.rs`
- Create: `crates/workmen-core/tests/scanner.rs`
- Create: `crates/workmen-core/tests/fixtures/projects/scan-game/assets/`

**Interfaces produced:** `ScanRequest`, `ScanResult`, `ScannedFile`, `ScanDiagnostic`, `ScanCache`.

```rust
pub struct ScanRequest<'a> { pub root: &'a ProjectRoot, pub config: Option<&'a ProjectConfig>, pub mode: ScanMode }
pub fn scan_project(request: ScanRequest<'_>) -> Result<ScanResult, WorkmenError>;
```

- [ ] Generate tiny synthetic PNG, JPG, WebP, and SVG fixtures in the test using the `image` crate; add contextual iOS `Contents.json`, Android vector/adaptive-icon XML, and runtime asset-manifest JS fixtures.
- [ ] Add non-art JSON/XML/JS fixtures and assert they are ignored.
- [ ] Add corrupt PNG and symlink fixtures; assert the corrupt file creates a diagnostic, the symlink is not followed, and valid assets still return.
- [ ] Implement walking with `ignore`, honoring `.gitignore`, built-in excludes (`.git`, dependencies, caches), and configurable project excludes. Preserve deprecated/rejected matches as visible excluded records.
- [ ] Recognize art metadata by path plus minimum structural markers, never extension alone.
- [ ] Decode image headers and metadata without retaining pixels after inspection. Compute decoded byte estimates safely with checked arithmetic.
- [ ] Add stat cache entries keyed by relative path, size, and modified time; hash changed content with BLAKE3. Store the cache under the OS app-cache directory and always hash candidate mirror relationships.
- [ ] Keep result ordering deterministic by normalized relative path, independent of Rayon scheduling.
- [ ] Run `rtk cargo test -p workmen-core --test scanner`; expect all formats, error isolation, exclude behavior, and deterministic order to pass.
- [ ] Commit: `rtk git add crates/workmen-core && rtk git commit -m "feat(scanner): discover game art and metadata"`.

### Task 5: Classify roles, infer families, and draft Profiles

**Files:**
- Create: `crates/workmen-core/src/classify/mod.rs`
- Create: `crates/workmen-core/src/classify/roles.rs`
- Create: `crates/workmen-core/src/classify/families.rs`
- Create: `crates/workmen-core/src/classify/draft.rs`
- Create: `crates/workmen-core/tests/classification.rs`

**Interfaces consumed:** `ScanResult`, optional `ProjectConfig`.
**Interfaces produced:** `ClassificationResult`, `RoleAssignment`, `DraftProfile`, `Confidence`, `UnclassifiedEntry`.

```rust
pub fn classify(scan: &ScanResult, config: Option<&ProjectConfig>, policy: &ClassificationPolicy) -> ClassificationResult;
pub fn draft_profiles(classification: &ClassificationResult) -> Vec<DraftProfile>;
```

- [ ] Write table-driven tests for Source, Runtime, Derived, Mirror Target, Excluded, and Unclassified classification using path, declared mapping, hash, and metadata evidence.
- [ ] Assert generated web/iOS/Android copies are linked as Mirror Targets to one Runtime asset rather than emitted as independent inventory roots.
- [ ] Assert low-confidence assets enter the Unclassified Queue and no role is guessed.
- [ ] Implement evidence scoring with named reasons. Keep thresholds in one typed `ClassificationPolicy`, not scattered numeric literals.
- [ ] Group candidate families by directory, naming stem/token shape, dimensions, format, and contextual metadata links. Never merge families solely because dimensions match.
- [ ] Produce Draft Profiles containing proposed matchers, observed ranges, representative assets, confidence, and unresolved conflicts. Do not write them during scan.
- [ ] Run `rtk cargo test -p workmen-core --test classification`.
- [ ] Commit: `rtk git add crates/workmen-core && rtk git commit -m "feat(core): classify assets and draft profiles"`.

### Task 6: Resolve Profile matches and compute Spec Diff

**Files:**
- Create: `crates/workmen-core/src/profile/mod.rs`
- Create: `crates/workmen-core/src/profile/matcher.rs`
- Create: `crates/workmen-core/src/profile/resolver.rs`
- Create: `crates/workmen-core/src/profile/lifecycle.rs`
- Create: `crates/workmen-core/src/validate/mod.rs`
- Create: `crates/workmen-core/src/validate/diff.rs`
- Create: `crates/workmen-core/src/validate/rules.rs`
- Create: `crates/workmen-core/tests/profile_resolution.rs`
- Create: `crates/workmen-core/tests/spec_diff.rs`

**Interfaces produced:** `ProfileResolver::resolve`, `ProfileLifecycle`, `Validator::validate_asset`.

```rust
impl ProfileResolver { pub fn resolve<'a>(&self, asset: &Asset, profiles: &'a [Profile]) -> Result<Option<&'a Profile>, ResolveError>; }
impl Validator { pub fn validate_asset(&self, asset: &Asset, profile: &Profile, platform: Platform) -> Vec<ValidationIssue>; }
```

- [ ] Write matcher tests covering role, path glob, naming pattern, extension, and combined specificity. Equal specificity must return `AmbiguousProfile` with both IDs.
- [ ] Write lifecycle tests: Draft may edit, activation validates, lock increments revision, Locked rejects edit, unlock requires a non-empty reason and increments revision.
- [ ] Write Spec Diff tests for dimensions, aspect ratio, frame/output count, alpha/background, padding/trim, color/bit depth, encoded bytes, decoded bytes, POT/NPOT, and Web/iOS/Android budgets.
- [ ] Implement deterministic specificity as an ordered tuple of explicit constraints, not a floating score. Document the tuple beside the resolver.
- [ ] Apply naming matchers before raising naming violations. Apply versioned exceptions only when rule, asset matcher, and unexpired date all match.
- [ ] Return one `ValidationIssue` per failed rule/platform with actual/expected values and suggested next action. Do not collapse cross-platform failures.
- [ ] Run `rtk cargo test -p workmen-core --test profile_resolution --test spec_diff`.
- [ ] Commit: `rtk git add crates/workmen-core && rtk git commit -m "feat(validation): resolve profiles and compute spec diff"`.

### Task 7: Add reports, logs, CLI behavior, and exit contracts

**Files:**
- Create: `crates/workmen-core/src/report/mod.rs`
- Create: `crates/workmen-core/src/report/text.rs`
- Create: `crates/workmen-core/src/report/json.rs`
- Create: `crates/workmen-core/src/report/sarif.rs`
- Create: `crates/workmen-core/src/logging.rs`
- Create: `crates/workmen-core/src/service.rs`
- Modify: `crates/workmen-cli/src/main.rs`
- Create: `crates/workmen-cli/tests/scan_validate.rs`
- Create: `crates/workmen-core/tests/reporting.rs`

**Interfaces produced:** `WorkmenService::{scan,validate}`, `ReportFormat`, CLI exit codes `0/1/2`.

```rust
impl WorkmenService {
    pub fn scan(&self, request: ScanProjectRequest) -> Result<ProjectSnapshot, WorkmenError>;
    pub fn validate(&self, request: ValidateProjectRequest) -> Result<ValidationReport, WorkmenError>;
}
```

- [ ] Write CLI integration tests for text scan output, JSON validation output, SARIF output, `--warnings-as-errors`, config failure exit `2`, validation error exit `1`, and pass exit `0`.
- [ ] Validate SARIF output against a checked-in minimal SARIF 2.1.0 structural fixture: version, driver rules, result rule IDs, levels, and relative artifact URIs.
- [ ] Implement `WorkmenService` as the only orchestration entry used by CLI now and by the subsequent Tauri plan. It returns domain results; frontends choose formatting.
- [ ] Configure structured JSONL rolling logs in the OS application-log directory with Info default, Debug opt-in, relative paths, ten-file/50-MB retention, stderr diagnostics, and CLI `--log-file` override.
- [ ] Ensure text reports are human-readable while JSON/SARIF stdout contains no log lines.
- [ ] Run `rtk cargo test --workspace` and manually run `rtk cargo run -p workmen-cli -- scan crates/workmen-core/tests/fixtures/projects/scan-game --format json`; expect stable asset records and no project changes.
- [ ] Commit: `rtk git add crates/workmen-core crates/workmen-cli && rtk git commit -m "feat(cli): expose scan and validation reports"`.

### Task 8: Prove Night Market Merge read-only compatibility and CI portability

**Files:**
- Create: `scripts/accept-night-market.sh`
- Create: `tests/acceptance/night-market.expected.json`
- Create: `.github/workflows/ci.yml`
- Modify: `README.md`

**Interfaces consumed:** `/Users/jojo/Github/project-merge-ios` supplied through `WORKMEN_NIGHT_MARKET_PATH`.

- [ ] Add a fixture-independent acceptance script that records `git status --porcelain` before and after scan, runs `workmen scan`, and fails if the target repository changes.
- [ ] Make the acceptance assertions contract-based rather than count-based: PNG/JPG/WebP/SVG discovered; contextual iOS/Android/JS metadata and native icon/splash bundles recognized; all six roles represented or explicitly reported as not yet configured; 14-by-4 atlas metadata found; deprecated/rejected excluded from shipping errors; missing Floating Market runtime files reported.
- [ ] Store only normalized rule/format expectations in `night-market.expected.json`; do not copy game art or absolute local paths.
- [ ] Add CI jobs for format, Clippy, tests, and native CLI builds on macOS, Windows, and Linux. Keep the live Night Market gate local because that repository is not a Workmen fixture.
- [ ] Run `WORKMEN_NIGHT_MARKET_PATH=/Users/jojo/Github/project-merge-ios rtk bash scripts/accept-night-market.sh`; expect a passing contract report and identical target `git status` before/after.
- [ ] Run `rtk cargo fmt --check && rtk cargo clippy --workspace --all-targets -- -D warnings && rtk cargo test --workspace`.
- [ ] Commit: `rtk git add scripts tests .github README.md && rtk git commit -m "test: gate core against Night Market assets"`.

## Plan Acceptance Gate

- [ ] `workmen scan <path>` is read-only and returns all supported art formats plus contextual art metadata.
- [ ] `workmen validate <path>` resolves Profiles deterministically and emits text, JSON, or SARIF.
- [ ] Exit codes, warnings-as-errors, JSONL logs, excludes, corrupt-file isolation, and ambiguous matching behave exactly as designed.
- [ ] Night Market Merge passes the live compatibility script without repository mutation.
- [ ] No desktop UI, asset transformation, arbitrary shell hook, or direct image-generation API has entered this slice.
