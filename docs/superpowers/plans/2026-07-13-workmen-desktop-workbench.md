# Workmen Desktop Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the cross-platform desktop workbench for opening and scanning projects, inspecting assets, reviewing Draft Profiles, locking project contracts, and navigating validation results and operation logs.

**Architecture:** A Tauri v2 shell invokes the same `WorkmenService` built in the core plan. React owns presentation state only; Rust commands own filesystem access and domain actions. A versioned JSON command boundary plus generated TypeScript contracts prevents GUI/CLI behavior drift.

**Tech Stack:** Tauri 2.11.x, React 19.2.x, TypeScript, Vite 8.1.x, Vitest, Testing Library, Playwright, Rust `tauri`, `serde`, `image`; Node 22.23.x.

## Global Constraints

- Complete `2026-07-13-workmen-core-validation.md` first.
- Never duplicate scanner, Profile, Spec Diff, validation, or reporting rules in TypeScript.
- The Inspector is read-only; every mutation routes to a named Tool or explicit project-contract command.
- Opening and scanning an uninitialized project creates no project files.
- Preserve keyboard access, visible focus, semantic labels, and non-color severity indicators.
- Use synthetic fixtures in automated tests. Night Market Merge remains a live local acceptance target.
- Every task begins with a failing test and ends with a focused commit.

## File Structure Map

- `apps/desktop/src-tauri/src/commands/`: permission-checked adapters into `WorkmenService`; no domain rules.
- `apps/desktop/src/lib/backend.ts`: the sole typed Tauri invocation/event adapter.
- `apps/desktop/src/features/project/`: project lifecycle and scan state.
- `apps/desktop/src/features/assets/` and `features/inspector/`: read-only inventory and inspection.
- `apps/desktop/src/features/profiles/`: Draft comparison and Profile lifecycle.
- `apps/desktop/src/features/console/`: Validation Results and Operations presentation.
- `packages/contracts/src/generated.ts`: generated Rust-to-TypeScript command/domain contracts.

---

### Task 1: Scaffold the Tauri application and typed command boundary

**Files:**
- Create: `package.json`
- Create: `apps/desktop/package.json`
- Create: `apps/desktop/index.html`
- Create: `apps/desktop/tsconfig.json`
- Create: `apps/desktop/vite.config.ts`
- Create: `apps/desktop/src/main.tsx`
- Create: `apps/desktop/src/App.tsx`
- Create: `apps/desktop/src/styles.css`
- Create: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/tauri.conf.json`
- Create: `apps/desktop/src-tauri/src/main.rs`
- Create: `apps/desktop/src-tauri/src/lib.rs`
- Create: `apps/desktop/src-tauri/src/commands/mod.rs`
- Create: `apps/desktop/src-tauri/src/commands/system.rs`
- Create: `packages/contracts/package.json`
- Create: `packages/contracts/src/generated.ts`
- Create: `crates/workmen-core/src/bindings.rs`
- Create: `apps/desktop/src/App.test.tsx`

**Interfaces consumed:** `WorkmenService`, serializable core models.
**Interfaces produced:** command envelope `{ apiVersion, requestId, data | error }`; generated TypeScript types.

```ts
export type CommandEnvelope<T> =
  | { apiVersion: 1; requestId: string; data: T }
  | { apiVersion: 1; requestId: string; error: { code: string; message: string } };

export interface WorkmenBackend {
  invoke<TRequest, TResponse>(command: string, request: TRequest): Promise<CommandEnvelope<TResponse>>;
}
```

- [ ] Write a React test that renders the shell name, reports backend availability, and displays a typed error envelope without exposing an absolute path.
- [ ] Write a Rust test that exports TypeScript declarations from core model types and fails when `packages/contracts/src/generated.ts` is stale.
- [ ] Configure the root npm workspace and scripts `desktop:dev`, `desktop:test`, `desktop:e2e`, `desktop:build`, and `contracts:check`.
- [ ] Add Tauri commands `get_system_info` and `get_app_log_directory`; expose only required capabilities in Tauri configuration.
- [ ] Build a four-region shell: project header, navigation rail, primary workspace, bottom console. Keep layout responsive down to 1024×700.
- [ ] Run `rtk npm install`, `rtk npm run contracts:check`, `rtk npm run desktop:test`, and `rtk cargo test --workspace`.
- [ ] Commit: `rtk git add package.json package-lock.json apps/desktop packages/contracts crates/workmen-core && rtk git commit -m "feat(desktop): bootstrap typed Tauri workbench"`.

### Task 2: Implement project open, scan progress, and inventory state

**Files:**
- Create: `apps/desktop/src-tauri/src/commands/project.rs`
- Create: `apps/desktop/src/lib/backend.ts`
- Create: `apps/desktop/src/features/project/project-store.ts`
- Create: `apps/desktop/src/features/project/OpenProject.tsx`
- Create: `apps/desktop/src/features/project/ProjectHeader.tsx`
- Create: `apps/desktop/src/features/project/ScanProgress.tsx`
- Create: `apps/desktop/src/features/project/OpenProject.test.tsx`
- Create: `apps/desktop/src/features/project/project-store.test.ts`

**Interfaces produced:** `open_project`, `scan_project`, `cancel_scan`, and event stream `scan://progress`.

```ts
export type ProjectPhase = "idle" | "opening" | "scanning" | "ready" | "failed" | "cancelled";
export type ScanProgress = { requestId: string; phase: string; completed: number; total: number | null; relativePath: string | null };
export type ProjectAction =
  | { type: "open"; requestId: string }
  | { type: "progress"; value: ScanProgress }
  | { type: "ready"; requestId: string; snapshot: ProjectSnapshot }
  | { type: "failed"; requestId: string; message: string };
```

- [ ] Write store tests for idle/opening/scanning/ready/failed/cancelled states and stale request rejection by `requestId`.
- [ ] Write component tests that show read-only status for an uninitialized project and require a separate Initialize action.
- [ ] Implement Rust commands as thin calls to `WorkmenService`; emit bounded progress events with phase, completed, total, and current relative path.
- [ ] Add native directory selection, recent-project entries stored in the app-data directory, rescan, and cancellation. Never persist thumbnail/index data in the game project.
- [ ] Show project root, config state, asset count, issue summary, scan duration, and explicit `Read-only scan` indicator.
- [ ] Verify cancellation leaves no partial project state and a subsequent scan succeeds.
- [ ] Run `rtk npm run desktop:test -- OpenProject project-store && rtk cargo test -p workmen-desktop project`.
- [ ] Commit: `rtk git add apps/desktop && rtk git commit -m "feat(desktop): open and scan game projects"`.

### Task 3: Build the Asset Browser

**Files:**
- Create: `apps/desktop/src/features/assets/AssetBrowser.tsx`
- Create: `apps/desktop/src/features/assets/AssetTree.tsx`
- Create: `apps/desktop/src/features/assets/AssetGrid.tsx`
- Create: `apps/desktop/src/features/assets/AssetCard.tsx`
- Create: `apps/desktop/src/features/assets/AssetFilters.tsx`
- Create: `apps/desktop/src/features/assets/asset-query.ts`
- Create: `apps/desktop/src/features/assets/asset-query.test.ts`
- Create: `apps/desktop/src/features/assets/AssetBrowser.test.tsx`
- Create: `apps/desktop/src-tauri/src/commands/thumbnail.rs`

**Interfaces consumed:** asset inventory and validation summary.
**Interfaces produced:** selected `AssetId`; thumbnail command returning cached app-local images.

```ts
export type AssetQuery = { roles: AssetRole[]; profileIds: string[]; formats: AssetFormat[]; severities: Severity[]; text: string };
export function queryAssets(assets: readonly Asset[], query: AssetQuery): Asset[];
```

- [ ] Write pure query tests for grouping/filtering by role, Profile, format, family, severity, and Unclassified state. Assert stable path ordering.
- [ ] Write interaction tests for tree/grid switch, multi-filter chips, keyboard selection, and preserving selection after incremental validation.
- [ ] Add app-local thumbnail generation keyed by content hash, requested size, and checker-background mode. Bound decoded image size and return a typed error for unsafe inputs.
- [ ] Implement virtualized grid rendering without loading original-resolution images into every card.
- [ ] Show role and severity with icon, text, and color; expose Excluded items without mixing them into shipping failures.
- [ ] Add Unclassified Queue and Mirror Target groups as first-class navigation entries.
- [ ] Run `rtk npm run desktop:test -- asset-query AssetBrowser && rtk cargo test -p workmen-desktop thumbnail`.
- [ ] Commit: `rtk git add apps/desktop && rtk git commit -m "feat(desktop): add asset browser and filters"`.

### Task 4: Build the read-only Asset Inspector and comparisons

**Files:**
- Create: `crates/workmen-core/src/inspect/mod.rs`
- Create: `crates/workmen-core/src/inspect/pixel_diff.rs`
- Create: `crates/workmen-core/tests/pixel_diff.rs`
- Create: `apps/desktop/src-tauri/src/commands/inspect.rs`
- Create: `apps/desktop/src/features/inspector/AssetInspector.tsx`
- Create: `apps/desktop/src/features/inspector/ImageViewport.tsx`
- Create: `apps/desktop/src/features/inspector/MetadataPanel.tsx`
- Create: `apps/desktop/src/features/inspector/ComparePanel.tsx`
- Create: `apps/desktop/src/features/inspector/AssetInspector.test.tsx`

**Interfaces produced:** `InspectionDetails`, `ComparisonRequest`, `ComparisonResult` with overlay and heatmap artifacts stored in app cache.

```rust
pub fn inspect_asset(request: InspectionRequest) -> Result<InspectionDetails, WorkmenError>;
pub fn compare_assets(request: ComparisonRequest) -> Result<ComparisonResult, WorkmenError>;
```

- [ ] Write Rust tests with synthetic pixels for identical, dimension-mismatch, alpha-only, and RGBA differences. Assert stable changed-pixel count and heatmap hash.
- [ ] Write UI tests for checker/background selection, zoom, pan, pixel grid, RGBA channels, Source/Runtime/Derived/Mirror selection, and issue-to-property focus.
- [ ] Implement guarded raster decode and SVG raster preview at Profile target resolutions. Report original metadata, alpha/trim bounds, encoded/decoded sizes, color type, and bit depth.
- [ ] Implement overlay, side-by-side, and pixel-diff heatmap using core comparison output. Do not put pixel math in React.
- [ ] Group exact content-hash duplicates in metadata and provide navigation among duplicate Asset IDs.
- [ ] Add reveal-in-file-manager, copy-relative-path, and open-in-external-editor actions with explicit user gesture.
- [ ] Ensure Inspector exposes no save/overwrite action and labels generated previews as temporary.
- [ ] Run `rtk cargo test -p workmen-core --test pixel_diff && rtk npm run desktop:test -- AssetInspector`.
- [ ] Commit: `rtk git add crates/workmen-core apps/desktop && rtk git commit -m "feat(desktop): add asset inspection and comparison"`.

### Task 5: Implement Draft Profile review and Profile Studio

**Files:**
- Create: `apps/desktop/src-tauri/src/commands/profile.rs`
- Create: `apps/desktop/src/features/profiles/ProfileStudio.tsx`
- Create: `apps/desktop/src/features/profiles/DraftReview.tsx`
- Create: `apps/desktop/src/features/profiles/ProfileForm.tsx`
- Create: `apps/desktop/src/features/profiles/YamlEditor.tsx`
- Create: `apps/desktop/src/features/profiles/MatcherPreview.tsx`
- Create: `apps/desktop/src/features/profiles/SpecDiffPanel.tsx`
- Create: `apps/desktop/src/features/profiles/ProfileStudio.test.tsx`
- Create: `crates/workmen-core/tests/profile_persistence.rs`

**Interfaces produced:** `preview_profile_change`, `save_draft_profile`, `activate_profile`, `lock_profile`, `unlock_profile`.

```rust
pub fn preview_profile_change(request: ProfileChangeRequest) -> Result<ProfileChangePreview, WorkmenError>;
pub fn save_draft_profile(request: SaveDraftRequest) -> Result<Profile, WorkmenError>;
pub fn lock_profile(request: LockProfileRequest) -> Result<Profile, WorkmenError>;
pub fn unlock_profile(request: UnlockProfileRequest) -> Result<Profile, WorkmenError>;
```

- [ ] Write persistence tests proving a Preset copy is independent, revisions increment, locked edits fail, unlock reason is recorded, and atomic YAML writes leave valid prior content after injected failure.
- [ ] Write UI tests for the approved flow: Auto-scan → Draft → Compare → Edit matcher/spec → Activate → Lock.
- [ ] Implement schema-backed form fields for matchers, dimensions, counts, alpha/background, padding/trim, color/bit depth, budgets, relationships, recipes, and severities.
- [ ] Add schema-aware YAML editing with parse errors and round-trip preservation. All saves call Rust validation before touching disk.
- [ ] Add Preset create/import/export. Creating a Profile copies Preset values and records provenance without retaining a mutable link.
- [ ] Show live match count, representative assets, conflicts, naming matcher effects, Spec Diff, and Unclassified assignment preview.
- [ ] Require a confirmation for initialization, a clean validation for activation, and a non-empty reason to unlock. Only Locked Profiles expose Generator Pack eligibility.
- [ ] Run `rtk cargo test -p workmen-core --test profile_persistence && rtk npm run desktop:test -- ProfileStudio`.
- [ ] Commit: `rtk git add crates/workmen-core apps/desktop && rtk git commit -m "feat(desktop): add Profile Studio lifecycle"`.

### Task 6: Build Validation Console Results and Operations

**Files:**
- Create: `apps/desktop/src-tauri/src/commands/validation.rs`
- Create: `apps/desktop/src-tauri/src/commands/operations.rs`
- Create: `apps/desktop/src/features/console/ValidationConsole.tsx`
- Create: `apps/desktop/src/features/console/ResultsView.tsx`
- Create: `apps/desktop/src/features/console/OperationsView.tsx`
- Create: `apps/desktop/src/features/console/IssueDetails.tsx`
- Create: `apps/desktop/src/features/console/ValidationConsole.test.tsx`

**Interfaces produced:** `validate_project`, `validate_assets`, `export_report`, `list_operation_events`.

```ts
export type ValidationFilter = { severities: Severity[]; profileIds: string[]; roles: AssetRole[]; platforms: Platform[] };
export function filterIssues(issues: readonly ValidationIssue[], filter: ValidationFilter): ValidationIssue[];
```

- [ ] Write tests for severity/Profile/type/role/platform grouping, Error/Warning/Info badges, keyboard navigation, and deterministic filter combinations.
- [ ] Assert selecting an issue navigates to the Asset, affected Inspector property, and appropriate Tool entry point.
- [ ] Implement incremental selected-asset validation and full-project validation with cancellation and progress.
- [ ] Add report export for text, JSON, and SARIF by delegating to core formatters. UI must not recreate report semantics.
- [ ] Display Operations separately from Results, with relative paths, input/output hashes where present, duration, outcome, and link to the OS log directory.
- [ ] Add explicit empty/pass/failure states and keep warnings visible even when they do not fail the run.
- [ ] Run `rtk npm run desktop:test -- ValidationConsole && rtk cargo test --workspace`.
- [ ] Commit: `rtk git add apps/desktop && rtk git commit -m "feat(desktop): add validation and operations console"`.

### Task 7: Add end-to-end, accessibility, and cross-platform release gates

**Files:**
- Create: `apps/desktop/playwright.config.ts`
- Create: `apps/desktop/e2e/open-scan-inspect.spec.ts`
- Create: `apps/desktop/e2e/profile-validation.spec.ts`
- Create: `apps/desktop/e2e/fixtures/synthetic-project.ts`
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`

**Interfaces consumed:** packaged desktop app and synthetic project generator.

- [ ] Add an E2E fixture builder that creates a temporary game project with valid, warning, corrupt, excluded, mirror, and unclassified cases.
- [ ] Test open → scan → filter → inspect → compare → issue navigation without project mutation.
- [ ] Test initialize confirmation → Draft review → naming matcher edit → Spec Diff update → activate → lock → validation pass.
- [ ] Add automated checks for keyboard-only primary flows, accessible names, focus restoration, reduced motion, and severity text independent of color.
- [ ] Extend CI to run TypeScript checks, Vitest, Rust tests, and Tauri build smoke tests on macOS, Windows, and Linux. Run Playwright UI flows on the supported CI display environment.
- [ ] Run `rtk npm run desktop:test && rtk npm run desktop:e2e && rtk npm run desktop:build` on macOS.
- [ ] Run a live read-only scan of `/Users/jojo/Github/project-merge-ios`; confirm the desktop inventory and CLI JSON agree on asset IDs, roles, Profile matches, and issue totals.
- [ ] Commit: `rtk git add apps/desktop .github README.md && rtk git commit -m "test(desktop): gate Workbench workflows"`.

## Plan Acceptance Gate

- [ ] A user can open any project, scan it read-only, browse all roles, inspect supported formats, and compare related outputs.
- [ ] Draft Profile review, naming comparison, activation, locking, and explicit reasoned unlock operate through validated project-local YAML.
- [ ] Validation Console separates Results from Operations and exports core-generated text, JSON, and SARIF.
- [ ] Desktop and CLI produce identical asset IDs, role assignments, Profile resolution, and issue totals.
- [ ] The packaged application builds on macOS, Windows, and Linux; automated tests use no Night Market artwork.
