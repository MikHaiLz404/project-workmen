# Workmen Design

**Status:** Approved during design interview on 2026-07-13

**Product name:** Workmen is an internal codename; public branding is deferred.

**Scope:** Standalone tooling for game art only.

## 1. Product Goal

Workmen is a cross-platform desktop application and CLI for discovering, inspecting, specifying, validating, slicing, and packing game art. It can open any game project, infer Draft Profiles from the assets it finds, compare actual files against explicit contracts, and export repeatable technical guidance for future asset generation.

The MVP must support every active art extension and art-specific metadata contract present in Night Market Merge. The current compatibility corpus includes PNG, JPG, WebP, SVG, contextual JSON/XML/JS metadata, fixed-grid atlases, chromakey source sheets, native app icons and splash bundles, layered scenes, runtime manifests, and generated mobile mirrors.

Workmen is not a painting, retouching, or image-generation application. It may perform explicit non-destructive pipeline operations, but it never replaces Photoshop-like creative editing and never calls an AI image generator in the MVP.

## 2. Architecture

Workmen consists of:

- a Tauri desktop shell;
- a React and TypeScript interface;
- a native CLI;
- one shared Rust core used by both GUI and CLI.

The Rust core owns project scanning, classification, Profile resolution, Spec Diff, validation, image operations, spritesheet slicing, atlas packing, report generation, and Generator Pack export. GUI and CLI parity is a release requirement.

```text
Desktop GUI (Tauri + React)        Native CLI
              \                     /
               \                   /
                  workmen-core
       scanner | classifier | profiles
       diff | validator | image operations
       spritesheets | atlases | reports | packs
```

The desktop application is installed locally and works offline. Distribution targets are macOS, Windows, and Linux. No server is required to scan or process a project.

## 3. Project Contract

Workmen opens a Game Project in read-only mode first. It does not create configuration until the user reviews the scan and selects `Initialize Project`.

Initialized projects use:

```text
.workmen/
├── project.yaml
└── specs/
    ├── item-icon.yaml
    ├── portrait.yaml
    ├── backdrop.yaml
    ├── atlas.yaml
    └── vfx.yaml
```

Files use YAML validated by JSON Schema. Every contract declares `schemaVersion`; every Profile declares `profileRevision`. Workmen caches thumbnails and indexes outside version control.

Presets are reusable starting points. Profiles are project-local contracts created from Presets or scan results. Changing a Preset never changes an existing Profile automatically.

Profile lifecycle is:

```text
Draft -> Compare -> Edit -> Activate -> Lock
```

A Locked Profile must be explicitly unlocked with a reason before editing. Profile changes increment `profileRevision` and are reviewed through Git.

## 4. Project Scanner

The Scanner accepts a Game Project path from the desktop application or `workmen scan <path>`.

It must:

1. locate the project root and read `.gitignore`;
2. apply built-in excludes for source-control internals, dependencies, build caches, deprecated paths, rejected art, and generated caches;
3. apply additional patterns from `.workmen/project.yaml`;
4. recognize PNG, JPG, WebP, and SVG images;
5. recognize JSON, XML, or JS only when context identifies the file as art metadata;
6. assign an Asset Role: Source, Runtime, Derived, Mirror Target, Excluded, or Unclassified;
7. inspect paths, naming patterns, dimensions, alpha, color information, and linked metadata;
8. group Assets into candidate Asset Families with confidence scores;
9. produce Draft Profiles and Spec Diffs for review.

Default excludes are configurable. Deprecated and rejected assets remain visible under Excluded but do not affect shipping pass/fail.

The Scanner does not follow symbolic links by default. A corrupt or unreadable file creates an Error without aborting the rest of the scan. It uses file size and modification time for its fast cache, then hashes changed files and Mirror Targets when content identity matters.

Assets with insufficient confidence enter the Unclassified Queue. Assigning one to a Profile can create a path or naming matcher for later scans. If multiple Profiles match, the most specific matcher wins; equal specificity creates `Ambiguous Profile` instead of guessing.

Draft Profile review follows:

```text
Auto-scan -> Draft Profile -> Compare -> Review differences
          -> Edit matcher or spec -> Lock Profile
```

Naming differences are evaluated through Naming Matchers before being treated as image violations.

## 5. Asset Browser and Inspector

The Asset Browser supports tree, grid, and grouped views by Asset Role, Profile, art type, and validation status.

The Inspector provides:

- transparent-checker or selectable-background previews;
- zoom, pan, pixel grid, and RGBA channel views;
- dimensions, aspect ratio, encoded size, and decoded-memory estimates;
- alpha bounds, transparent padding, trim bounds, color profile, and bit-depth information;
- SVG previews at target resolutions;
- Source/Runtime/Derived/Mirror comparisons;
- overlay, side-by-side, and pixel-diff heatmap modes;
- content-hash duplicate detection;
- file reveal, path copy, and external-editor actions;
- navigation from a Validation Issue to the affected Asset and property.

The Inspector is read-only. It sends changes to an appropriate Tool.

## 6. Profile Studio

A Profile binds an Asset Family to:

- path glob, naming pattern, extension, and Asset Role matchers;
- dimensions, aspect ratio, frame count, and output count;
- alpha, background, padding, trim, color space, and bit depth;
- encoded-size and decoded-memory Platform Budgets;
- Source-to-Runtime relationships;
- declarative Derived Export recipes;
- spritesheet and atlas layout rules;
- rule-specific validation severity.

Profile Studio shows live match counts, representative Assets, conflicting matchers, and Spec Diff before activation. It offers a form interface and schema-aware YAML editor. Presets can be created, imported, and exported.

Only a Locked Profile may be exported as a Generator Pack.

## 7. Spec Diff and Validation Console

Spec Diff compares a resolved Profile with an actual Asset. Every difference records expected value, actual value, rule ID, affected platform, severity, and suggested next action.

Validation severities are:

- **Error:** contract violation; CLI exits non-zero;
- **Warning:** usable but risky;
- **Info:** recommendation.

The Validation Console has Results and Operations views. It supports filtering and grouping by severity, Profile, type, role, and platform; incremental and full-project validation; and navigation into the relevant Tool.

Exceptions are version-controlled Profile entries with a reason and optional expiry. Local-only hiding of an issue is not allowed.

CLI exit codes are:

- `0`: validation passed;
- `1`: validation errors;
- `2`: configuration or tool failure.

`--warnings-as-errors` is available for release gates. Reports export as human-readable text, JSON, or SARIF.

Change Diff between historical scans is outside the MVP; Git remains the source of change history.

## 8. Logging

Operation logs are separate from validation reports. Workmen writes structured JSONL rolling logs to the operating system's application-log directory, not to the Game Project.

Logs use relative project paths and never contain image content. Info is the default level; temporary Debug logging is opt-in. The CLI writes diagnostics to stderr and accepts `--log-file`.

Default retention is ten files or 50 MB, deleting the oldest files first.

## 9. Sprite Sheet Studio

Sprite Sheet Studio supports uniform grids, horizontal and vertical strips, irregular frames, chromakey source sheets, and animation or VFX sequences.

It provides:

- automatic grid or region detection using alpha or chromakey;
- draggable guides for frame-bound correction;
- chromakey color selection, tolerance, and edge preview;
- Trim, Pad, Align, and Normalize Tools;
- frame reordering and animation groups;
- duration/FPS, loop, pivot/anchor, and trim offsets;
- Profile overlay and Expected/Actual comparison;
- real-time animation preview;
- export of individual frames, normalized sheets, and metadata.

All operations show before/after previews and produce Derived Assets. Collision shapes and hitboxes are deferred to a later version.

## 10. Atlas Studio

Atlas Studio supports Fixed Grid and Packed Atlas Profiles.

Fixed Grid defines columns, rows, cell size, and stable slot IDs. Empty locked slots remain transparent tombstones so existing runtime indexes do not move.

Packed Atlas uses deterministic MaxRects packing with configurable trim, padding, extrusion or edge bleed, optional rotation, POT/NPOT policy, maximum texture size, and multi-page policy.

Layout Lock preserves existing sprite slots or rectangles. New sprites use available space first. Full repack requires explicit confirmation and a preview of changed rectangles or UVs.

Atlas inspection shows frame bounds, pivots, overlap, clipping, transparent waste, packing density, decoded GPU-memory estimates, and Platform Budget violations.

Atlas export supports PNG or WebP according to Profile, generic JSON metadata, and template-based metadata such as a runtime JS manifest. Every exported entry retains a mapping to its Source Asset.

## 11. Derived Export

Profiles express non-destructive Export Recipes as ordered operations, for example:

```text
decode -> trim -> resize -> pad -> convert -> validate -> write
```

Export supports preview and dry-run. Work occurs in a staging directory; outputs are validated before any target changes. Commits to target paths are atomic, and failed jobs leave no partial set. Manifests are written last.

Source Assets are never overwritten. An existing Runtime or Derived target may be replaced only after explicit approval. Batch export is deterministic and records input/output hashes in the operation log.

Mirror Targets synchronize only after Runtime export succeeds. Workmen shows all added, changed, and removed files before committing the export.

MVP Profiles cannot run arbitrary shell commands.

## 12. Mirror Targets

Generated web and native copies are Mirror Targets rather than duplicate inventory items. A Mirror Target Profile maps each Runtime Asset to one or more destinations and validates content hashes after synchronization.

Night Market Merge compatibility includes its web directory, iOS public directory, and Android public directory. A stale mirror is a validation problem even when the Runtime Asset itself is valid.

## 13. Generator Pack

Workmen does not call an image generator in the MVP. It exports a provider-neutral folder or ZIP:

```text
generator-pack/
├── brief.md
├── constraints.yaml
├── expected-outputs.yaml
├── naming-map.csv
├── guides/
│   ├── canvas-template.png
│   ├── frame-grid.png
│   └── safe-area-overlay.png
└── references/
```

The pack records `packId`, Locked Profile ID and revision, dimensions, alpha/background rules, padding, frame layout, naming, expected count, animation metadata, atlas slots, Platform Budgets, optional art-direction notes, references, and negative constraints.

Import matches outputs to `packId`, verifies the Profile revision, maps filenames, produces Spec Diff, and routes valid results to Sprite Sheet Studio or Atlas Studio.

Workmen validates technical contracts. It does not claim to evaluate artistic quality or aesthetic appeal automatically.

## 14. Supported Formats and Contextual Metadata

The Night Market Merge tracked corpus currently contains:

| Extension | Tracked count at design time | Treatment |
|---|---:|---|
| PNG | 404 | Raster art, sprites, atlases, icons, portraits, UI, splash |
| JPG | 35 | Backdrops and previews |
| WebP | 18 | Runtime-optimized assets |
| SVG | 2 | Vector design/wireframe assets |

Counts include active, historical, documentation, and platform copies and are evidence for format coverage, not hard-coded expected totals.

Contextual metadata includes iOS Asset Catalog JSON, Android vector/adaptive-icon XML, runtime asset-manifest JS, and project-specific art manifests. Workmen must recognize these by path and schema; it must not classify every JSON, XML, or JS file as art.

## 15. Platform Budgets

Platform Budgets are mandatory Profile data. They can define target platform, maximum texture dimensions, encoded file size, decoded memory, color space, alpha policy, compression, and POT/NPOT constraints.

The initial targets are Web, iOS, and Android. One Asset Family may have distinct Runtime variants for different targets while preserving a single Source relationship.

## 16. Safety and Error Handling

- Project scan is read-only.
- Initialization requires explicit confirmation.
- Source Assets are immutable to Workmen.
- Transformations produce previewable Derived Assets.
- Full atlas repack requires explicit approval.
- Symlinks are not followed by default.
- One corrupt Asset does not abort the project scan.
- Ambiguous Profile resolution produces an Error instead of a guess.
- Export is staged, validated, and atomic.
- Project Profiles cannot execute arbitrary commands in the MVP.
- Logs omit image content and use project-relative paths.

## 17. Testing and MVP Acceptance

Automated coverage includes:

- Rust unit tests for Scanner, matchers, Spec Diff, validators, and packers;
- golden tests for resize, trim, slice, chromakey, and atlas output;
- determinism tests requiring identical input and Profile to produce identical hashes;
- corrupt, oversized, missing, and ambiguous Asset cases;
- Profile schema migration tests;
- GUI/CLI parity tests;
- SARIF schema validation;
- atomic-export failure and rollback tests;
- macOS, Windows, and Linux build CI.

Night Market Merge is the live compatibility gate. Workmen must:

- scan its PNG, JPG, WebP, and SVG files plus contextual art metadata;
- classify Source, Runtime, Derived, Mirror Target, Excluded, and Unclassified roles;
- inspect native app icons and splash bundles;
- understand the 14-by-4, 56-slot item atlas;
- validate Source-to-Runtime-to-mobile-mirror relationships;
- report missing Floating Market runtime-contract files;
- keep deprecated and rejected art out of shipping failure results;
- complete the scan without modifying the repository.

Workmen's checked-in automated fixtures use synthetic images. Final compatibility acceptance scans the real Night Market Merge repository so game artwork is not copied into Workmen.

## 18. MVP Modules

1. Project Scanner
2. Asset Browser and Inspector
3. Profile Studio
4. Validation Console
5. Sprite Sheet Studio
6. Atlas Studio
7. Derived Export
8. Generator Pack Export

## 19. Deferred Scope

- collision shapes and hitboxes;
- historical Change Diff between scans;
- direct AI image-generator APIs;
- painting, retouching, or aesthetic scoring;
- arbitrary project shell hooks;
- public product naming and branding;
- engine-specific adapters beyond contextual metadata and template exporters;
- mobile versions of the Workmen application.
