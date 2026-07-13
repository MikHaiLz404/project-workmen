# Workmen Sprite, Atlas, Export, and Generator Pack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver deterministic, non-destructive sprite-sheet and atlas workflows, atomic Derived/Runtime/Mirror export, and provider-neutral Generator Packs that lock future generated assets to approved game-art contracts.

**Architecture:** `workmen-core` gains a declarative operation graph, image-processing services, sprite and atlas domain modules, and a transactional export coordinator. React studios call preview/execute commands through the existing typed boundary. Every output is reproducible from Source hash + Locked Profile revision + recipe.

**Tech Stack:** Existing Rust/Tauri/React workspace, Rust `image`, `imageproc`, `png`, `webp`, `blake3`, `uuid`, `csv`, `zip`, `serde_yaml`, golden-image and property tests, Vitest and Playwright.

## Global Constraints

- Complete the core-validation and desktop-workbench plans first.
- Source Assets are immutable. Preview uses app cache; committed results are Derived, Runtime, or Mirror Target assets.
- Exports stage, validate, and atomically commit; manifests are written last. An injected failure must leave the previous target set intact.
- Existing Runtime/Derived targets require explicit overwrite approval. Full atlas repack requires separate explicit approval and a UV/rectangle diff.
- The same inputs and Locked Profile revision must produce byte-identical outputs where the encoder permits it and identical semantic hashes otherwise.
- No arbitrary shell commands, direct AI provider API, aesthetic scoring, collision shape, or hitbox generation.
- Begin every behavior with a failing focused test and commit each task independently.

## File Structure Map

- `crates/workmen-core/src/operation/` and `src/image_ops/`: declarative, bounded transformations and preview artifacts.
- `crates/workmen-core/src/export/` and `src/mirror/`: transactional plans, staging, commit, rollback, and parity.
- `crates/workmen-core/src/sprite/`: sheet detection, slicing, timing, pivots, and metadata.
- `crates/workmen-core/src/atlas/`: Fixed Grid, deterministic MaxRects, Layout Lock, metrics, rendering, and mappings.
- `crates/workmen-core/src/generator_pack/`: provider-neutral pack export/import and guide generation.
- `apps/desktop/src/features/sprite/`, `atlas/`, `operations/`, and `generator-pack/`: previews and explicit approvals only.

---

### Task 1: Implement the declarative operation graph and preview artifacts

**Files:**
- Create: `crates/workmen-core/src/operation/mod.rs`
- Create: `crates/workmen-core/src/operation/recipe.rs`
- Create: `crates/workmen-core/src/operation/executor.rs`
- Create: `crates/workmen-core/src/operation/artifact.rs`
- Create: `crates/workmen-core/src/image_ops/mod.rs`
- Create: `crates/workmen-core/src/image_ops/trim.rs`
- Create: `crates/workmen-core/src/image_ops/resize.rs`
- Create: `crates/workmen-core/src/image_ops/pad.rs`
- Create: `crates/workmen-core/src/image_ops/convert.rs`
- Create: `crates/workmen-core/tests/operation_recipe.rs`
- Create: `crates/workmen-core/tests/image_operations.rs`
- Create: `crates/workmen-core/tests/golden/image-operations/`

**Interfaces produced:** `ExportRecipe`, `Operation::{Decode,Trim,Resize,Pad,Align,Convert,Validate,Write}`, `PreviewArtifact`, `OperationExecutor`.

```rust
impl OperationExecutor {
    pub fn preview(&self, source: &Asset, profile: &Profile, recipe: &ExportRecipe) -> Result<PreviewArtifact, WorkmenError>;
}
```

- [ ] Write parser tests accepting only the declared operations in a legal order and rejecting `Write` before `Validate`, unknown operations, unsafe paths, and shell-like entries.
- [ ] Create tiny synthetic golden images covering transparent trim, no-alpha trim, nearest-neighbor pixel-art resize, padded alignment, and PNG/WebP conversion.
- [ ] Implement checked dimensions and decoded-memory limits before allocation. Preserve color/alpha semantics declared by Profile.
- [ ] Derive preview cache keys from Source hash, Profile ID/revision, recipe, target platform, and tool version.
- [ ] Return a manifest containing input/output hashes, dimensions, relative proposed path, warnings, and temporary preview location. Do not modify the game project.
- [ ] Run `rtk cargo test -p workmen-core --test operation_recipe --test image_operations` twice and assert stable hashes.
- [ ] Commit: `rtk git add crates/workmen-core && rtk git commit -m "feat(pipeline): add declarative image operations"`.

### Task 2: Add transactional Derived and Runtime export

**Files:**
- Create: `crates/workmen-core/src/export/mod.rs`
- Create: `crates/workmen-core/src/export/plan.rs`
- Create: `crates/workmen-core/src/export/staging.rs`
- Create: `crates/workmen-core/src/export/commit.rs`
- Create: `crates/workmen-core/tests/atomic_export.rs`
- Create: `apps/desktop/src-tauri/src/commands/export.rs`
- Create: `apps/desktop/src/features/operations/ExportPreview.tsx`
- Create: `apps/desktop/src/features/operations/ExportPreview.test.tsx`

**Interfaces produced:** `ExportPlan`, `ExportDiff::{Added,Changed,Removed}`, `ExportApproval`, `ExportOutcome`.

```rust
impl ExportCoordinator {
    pub fn plan(&self, request: ExportRequest) -> Result<ExportPlan, WorkmenError>;
    pub fn commit(&self, plan: ExportPlan, approval: ExportApproval) -> Result<ExportOutcome, WorkmenError>;
}
```

- [ ] Write tests for dry-run, successful commit, explicit overwrite approval, manifest-last order, injected write/validation/rename failure, rollback, and cleanup of abandoned staging.
- [ ] Assert a Source target is always rejected even with overwrite approval.
- [ ] Build an Export Plan that resolves all paths before decoding, rejects escapes outside declared roots, and shows every add/change/remove with hashes.
- [ ] Stage into a same-filesystem sibling directory, validate staged artifacts, preserve the current target as rollback material, then atomically swap. Record the completed operation only after final verification.
- [ ] Implement UI preview with before/after thumbnails, path and hash diff, Profile revision, validation summary, and separate overwrite confirmation.
- [ ] Emit progress/cancellation events; cancellation before commit removes staging, while commit itself is an uninterruptible short transaction.
- [ ] Run `rtk cargo test -p workmen-core --test atomic_export && rtk npm run desktop:test -- ExportPreview`.
- [ ] Commit: `rtk git add crates/workmen-core apps/desktop && rtk git commit -m "feat(pipeline): add atomic derived export"`.

### Task 3: Implement sprite-sheet detection, slicing, and metadata

**Files:**
- Create: `crates/workmen-core/src/sprite/mod.rs`
- Create: `crates/workmen-core/src/sprite/model.rs`
- Create: `crates/workmen-core/src/sprite/detect.rs`
- Create: `crates/workmen-core/src/sprite/chromakey.rs`
- Create: `crates/workmen-core/src/sprite/slice.rs`
- Create: `crates/workmen-core/src/sprite/metadata.rs`
- Create: `crates/workmen-core/tests/sprite_sheet.rs`
- Create: `crates/workmen-core/tests/sprite_sheet_golden.rs`
- Create: `crates/workmen-core/tests/golden/sprite-sheet/`

**Interfaces produced:** `SheetMode`, `FrameRegion`, `AnimationGroup`, `SpriteSheetSpec`, `SpriteSheetResult`.

```rust
pub fn detect_sheet(source: &DecodedImage, request: DetectionRequest) -> Result<DetectionProposal, WorkmenError>;
pub fn slice_sheet(source: &DecodedImage, spec: &SpriteSheetSpec) -> Result<SpriteSheetResult, WorkmenError>;
```

- [ ] Create synthetic fixtures for uniform grid, horizontal strip, vertical strip, irregular alpha regions, chromakey with tolerance boundary, and empty frame.
- [ ] Test automatic grid/region proposals separately from accepted guide edits. Detection never becomes the saved contract without review.
- [ ] Implement chromakey in a declared color space with exact tolerance semantics and edge preview mask; persist chosen key/tolerance in the Profile.
- [ ] Implement trim, pad, align, normalize, reorder, frame groups, FPS or per-frame duration, loop, pivot/anchor, and trim offsets.
- [ ] Export individual frames, normalized sheets, and generic JSON metadata that maps every frame to Source Asset and region.
- [ ] Add property tests that slicing then rebuilding an untrimmed uniform sheet reproduces the source pixels.
- [ ] Run `rtk cargo test -p workmen-core --test sprite_sheet --test sprite_sheet_golden`.
- [ ] Commit: `rtk git add crates/workmen-core && rtk git commit -m "feat(sprite): detect and slice sprite sheets"`.

### Task 4: Build Sprite Sheet Studio

**Files:**
- Create: `apps/desktop/src-tauri/src/commands/sprite.rs`
- Create: `apps/desktop/src/features/sprite/SpriteSheetStudio.tsx`
- Create: `apps/desktop/src/features/sprite/SheetCanvas.tsx`
- Create: `apps/desktop/src/features/sprite/GuideEditor.tsx`
- Create: `apps/desktop/src/features/sprite/FrameList.tsx`
- Create: `apps/desktop/src/features/sprite/AnimationPreview.tsx`
- Create: `apps/desktop/src/features/sprite/ChromakeyControls.tsx`
- Create: `apps/desktop/src/features/sprite/SpriteSheetStudio.test.tsx`

**Interfaces consumed:** sprite detection/slicing service and Export Plan.

- [ ] Write UI tests for mode selection, auto-detect review, draggable guide correction, chromakey picker/tolerance, frame reorder, group assignment, playback timing, pivot, and Profile expected/actual overlay.
- [ ] Implement preview commands with request cancellation so stale detection results cannot replace newer guide edits.
- [ ] Display original, keyed/alpha, normalized, and animation previews without writing project files.
- [ ] Route Trim, Pad, Align, and Normalize through the operation graph and show resulting Spec Diff.
- [ ] Export through the common Export Preview; require Locked Profile for Runtime output while allowing Draft-derived temporary previews.
- [ ] Add keyboard frame navigation, numeric guide entry, zoom/pan, and reduced-motion playback behavior.
- [ ] Run `rtk npm run desktop:test -- SpriteSheetStudio && rtk cargo test -p workmen-core sprite`.
- [ ] Commit: `rtk git add apps/desktop && rtk git commit -m "feat(desktop): add Sprite Sheet Studio"`.

### Task 5: Implement Fixed Grid atlas contracts and export

**Files:**
- Create: `crates/workmen-core/src/atlas/mod.rs`
- Create: `crates/workmen-core/src/atlas/model.rs`
- Create: `crates/workmen-core/src/atlas/fixed_grid.rs`
- Create: `crates/workmen-core/src/atlas/render.rs`
- Create: `crates/workmen-core/src/atlas/metadata.rs`
- Create: `crates/workmen-core/tests/fixed_grid_atlas.rs`

**Interfaces produced:** `AtlasSpec`, `FixedGridSpec`, `AtlasSlot`, `AtlasPage`, `AtlasEntry`, `SourceMapping`.

```rust
pub fn build_fixed_grid(inputs: &[AtlasInput], spec: &FixedGridSpec) -> Result<AtlasBuild, WorkmenError>;
```

- [ ] Write a 14-column × 4-row, 56-slot synthetic contract test matching Night Market's item-atlas shape without copying its artwork.
- [ ] Test stable slot IDs, transparent tombstones, slot collision, clipping, pivot preservation, and deterministic raster/metadata hashes.
- [ ] Implement cell placement with explicit fit policy; never silently resize or crop a sprite to hide a violation.
- [ ] Render PNG/WebP according to Profile and generic JSON with page, rectangle, source size, trim offset, pivot, slot ID, and Source mapping.
- [ ] Add template-based JS metadata rendering with escaped values and a constrained variable set; templates cannot execute code or access the filesystem.
- [ ] Run `rtk cargo test -p workmen-core --test fixed_grid_atlas` twice and compare output hashes.
- [ ] Commit: `rtk git add crates/workmen-core && rtk git commit -m "feat(atlas): add stable fixed-grid atlases"`.

### Task 6: Implement deterministic packed atlases and Layout Lock

**Files:**
- Create: `crates/workmen-core/src/atlas/maxrects.rs`
- Create: `crates/workmen-core/src/atlas/layout_lock.rs`
- Create: `crates/workmen-core/src/atlas/metrics.rs`
- Create: `crates/workmen-core/tests/packed_atlas.rs`
- Create: `crates/workmen-core/tests/layout_lock.rs`

**Interfaces produced:** `PackedAtlasSpec`, `LayoutLock`, `LayoutDiff`, `PackingMetrics`.

```rust
pub fn pack_atlas(inputs: &[AtlasInput], spec: &PackedAtlasSpec, lock: Option<&LayoutLock>) -> Result<AtlasBuild, PackError>;
pub fn diff_layout(previous: &LayoutLock, next: &AtlasBuild) -> LayoutDiff;
```

- [ ] Write deterministic MaxRects tests with shuffled input order; normalize by stable Asset ID before packing and require identical rectangles/pages.
- [ ] Test trim, padding, extrusion, optional rotation, POT/NPOT, maximum texture size, multi-page, oversized sprite, and zero-area sprite.
- [ ] Test Layout Lock: existing rectangles remain unchanged, new sprites use available space, removed sprites leave reusable locked space according to policy, and impossible insertion reports a conflict.
- [ ] Implement full-repack preview returning every changed page/rectangle/rotation/UV and decoded-memory delta. Execution requires `approveFullRepack: true` independent of overwrite approval.
- [ ] Compute packing density, transparent waste, overlap, clipping, and per-platform GPU-memory estimates as Validation Issues.
- [ ] Add fuzz/property tests asserting no overlaps, in-bounds rectangles, stable mappings, and finite termination.
- [ ] Run `rtk cargo test -p workmen-core --test packed_atlas --test layout_lock`.
- [ ] Commit: `rtk git add crates/workmen-core && rtk git commit -m "feat(atlas): add deterministic packed layouts"`.

### Task 7: Build Atlas Studio

**Files:**
- Create: `apps/desktop/src-tauri/src/commands/atlas.rs`
- Create: `apps/desktop/src/features/atlas/AtlasStudio.tsx`
- Create: `apps/desktop/src/features/atlas/AtlasCanvas.tsx`
- Create: `apps/desktop/src/features/atlas/AtlasSettings.tsx`
- Create: `apps/desktop/src/features/atlas/SlotList.tsx`
- Create: `apps/desktop/src/features/atlas/LayoutDiffPanel.tsx`
- Create: `apps/desktop/src/features/atlas/AtlasMetrics.tsx`
- Create: `apps/desktop/src/features/atlas/AtlasStudio.test.tsx`

**Interfaces consumed:** fixed/packed atlas services, Layout Lock, Export Plan.

- [ ] Write UI tests for Fixed Grid/Packed switch, slot assignment, tombstone display, rectangle/pivot overlay, metrics, Profile violations, and multipage navigation.
- [ ] Test that adding an asset under Layout Lock preserves existing rectangles and that full repack cannot execute from ordinary export confirmation.
- [ ] Implement real-time pack preview in a cancellable background command. Display bounds, pivots, extrusion, clipping, transparent waste, and Source mapping.
- [ ] Display Layout Diff before full repack, including changed UVs and runtime-risk warning.
- [ ] Export atlas pages and metadata through common atomic export, with manifests last.
- [ ] Run `rtk npm run desktop:test -- AtlasStudio && rtk cargo test -p workmen-core atlas`.
- [ ] Commit: `rtk git add apps/desktop && rtk git commit -m "feat(desktop): add Atlas Studio"`.

### Task 8: Export and import provider-neutral Generator Packs

**Files:**
- Create: `crates/workmen-core/src/generator_pack/mod.rs`
- Create: `crates/workmen-core/src/generator_pack/export.rs`
- Create: `crates/workmen-core/src/generator_pack/import.rs`
- Create: `crates/workmen-core/src/generator_pack/guides.rs`
- Create: `crates/workmen-core/tests/generator_pack.rs`
- Create: `apps/desktop/src-tauri/src/commands/generator_pack.rs`
- Create: `apps/desktop/src/features/generator-pack/GeneratorPackPanel.tsx`
- Create: `apps/desktop/src/features/generator-pack/GeneratorPackPanel.test.tsx`

**Interfaces produced:** `GeneratorPackManifest`, folder/ZIP exporter, `GeneratorImportResult`.

```rust
pub fn export_generator_pack(profile: &Profile, request: PackExportRequest) -> Result<GeneratorPackManifest, WorkmenError>;
pub fn inspect_generator_outputs(profile: &Profile, request: PackImportRequest) -> Result<GeneratorImportResult, WorkmenError>;
```

- [ ] Write tests rejecting Draft/Active Profiles and accepting only Locked Profiles with stable ID and revision.
- [ ] Assert folder and ZIP contain `brief.md`, `constraints.yaml`, `expected-outputs.yaml`, `naming-map.csv`, required guides, and explicitly selected references.
- [ ] Generate canvas, grid, and safe-area guide PNGs from technical constraints. Keep optional art direction and negative constraints distinct from technical rules.
- [ ] Import by `packId`, verify Profile ID/revision, map filenames, and produce Spec Diff without moving files. Unknown/missing/duplicate output names remain reviewable errors.
- [ ] Route valid imports to Sprite Sheet Studio or Atlas Studio based on Profile; never claim aesthetic correctness.
- [ ] Add zip-slip/path traversal, oversized archive, unsupported format, and corrupted manifest tests.
- [ ] Run `rtk cargo test -p workmen-core --test generator_pack && rtk npm run desktop:test -- GeneratorPackPanel`.
- [ ] Commit: `rtk git add crates/workmen-core apps/desktop && rtk git commit -m "feat(packs): lock generation to Profile contracts"`.

### Task 9: Synchronize Mirror Targets after successful Runtime export

**Files:**
- Create: `crates/workmen-core/src/mirror/mod.rs`
- Create: `crates/workmen-core/src/mirror/plan.rs`
- Create: `crates/workmen-core/src/mirror/verify.rs`
- Create: `crates/workmen-core/tests/mirror_sync.rs`
- Modify: `crates/workmen-core/src/export/commit.rs`
- Modify: `apps/desktop/src/features/operations/ExportPreview.tsx`

**Interfaces produced:** `MirrorPlan`, `MirrorDiff`, `MirrorVerification`.

```rust
pub fn plan_mirrors(runtime: &ExportOutcome, config: &MirrorConfig) -> Result<MirrorPlan, WorkmenError>;
pub fn verify_mirrors(plan: &MirrorPlan) -> Result<MirrorVerification, WorkmenError>;
```

- [ ] Write tests proving mirrors do not run when Runtime validation/export fails and do run only after Runtime commit succeeds.
- [ ] Test added/changed/removed preview, hash parity, stale mirror validation, rollback after mirror failure, and multiple destinations.
- [ ] Map one Runtime Asset to web, iOS public, and Android public destinations without duplicating inventory identity.
- [ ] Extend Export Preview with a separate Mirror section and final target hashes. Apply the full Runtime+Mirror change as one recoverable transaction.
- [ ] Record relative source/target paths and hashes in operation logs, never image content.
- [ ] Run `rtk cargo test -p workmen-core --test mirror_sync && rtk npm run desktop:test -- ExportPreview`.
- [ ] Commit: `rtk git add crates/workmen-core apps/desktop && rtk git commit -m "feat(pipeline): synchronize verified mirror targets"`.

### Task 10: Complete Night Market acceptance and release gates

**Files:**
- Create: `tests/acceptance/night-market-profile/`
- Create: `apps/desktop/e2e/sprite-atlas-export.spec.ts`
- Create: `scripts/accept-night-market-pipeline.sh`
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`

**Interfaces consumed:** real Night Market project via `WORKMEN_NIGHT_MARKET_PATH`; synthetic checked-in equivalents for CI.

- [ ] Define project-local acceptance Profiles outside the game repository for PNG/JPG/WebP/SVG inspection, chromakey sheet, 14×4 fixed atlas, Runtime mapping, and web/iOS/Android mirrors.
- [ ] Run read-only scan/validation against `/Users/jojo/Github/project-merge-ios` and assert no repository mutation.
- [ ] Export all transformation results to a temporary sandbox cloned from its directory structure; never overwrite the live project during acceptance.
- [ ] Verify sprite previews, fixed atlas slot stability, Runtime-to-mirror hash parity, deprecated/rejected exclusion, missing Floating Market runtime-contract report, and Generator Pack round trip.
- [ ] Add E2E coverage for sheet preview → Spec Diff → export preview and atlas Layout Lock → add sprite → stable export.
- [ ] Extend cross-platform CI with golden hashes, determinism reruns, atomic rollback, Generator Pack safety, Tauri build, and synthetic E2E pipeline.
- [ ] Run `WORKMEN_NIGHT_MARKET_PATH=/Users/jojo/Github/project-merge-ios rtk bash scripts/accept-night-market-pipeline.sh` and verify before/after `git status --porcelain` is identical.
- [ ] Run the complete release gate: `rtk cargo fmt --check && rtk cargo clippy --workspace --all-targets -- -D warnings && rtk cargo test --workspace && rtk npm run desktop:test && rtk npm run desktop:e2e && rtk npm run desktop:build`.
- [ ] Commit: `rtk git add tests apps/desktop/e2e scripts .github README.md && rtk git commit -m "test: accept Workmen pipeline against Night Market"`.

## Plan Acceptance Gate

- [ ] Sprite Sheet Studio handles uniform, strip, irregular, chromakey, animation, and VFX sheets with reviewed guides and deterministic Derived outputs.
- [ ] Atlas Studio handles stable Fixed Grid and deterministic Packed Atlas workflows, Layout Lock, multipage export, and explicit full-repack approval.
- [ ] All writes use preview, validation, staging, atomic commit, manifest-last ordering, and rollback; Source Assets are never overwritten.
- [ ] Generator Packs lock technical output contracts to a Locked Profile revision and round-trip through Spec Diff without provider coupling.
- [ ] Runtime exports synchronize verified web/iOS/Android Mirror Targets only after success.
- [ ] Night Market live acceptance completes without copying its art into Workmen or changing its repository.
