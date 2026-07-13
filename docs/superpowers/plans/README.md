# Workmen MVP Implementation Sequence

The approved Workmen design is implemented in three ordered, independently testable slices:

1. [Core, Scanner, Profiles, and Validation](./2026-07-13-workmen-core-validation.md) — shared Rust domain core, native CLI, read-only discovery, contracts, Spec Diff, reports, and Night Market scan gate.
2. [Desktop Workbench](./2026-07-13-workmen-desktop-workbench.md) — Tauri/React Asset Browser, Inspector, Profile Studio, and Validation Console using the shared core.
3. [Sprite, Atlas, Export, and Generator Pack](./2026-07-13-workmen-sprite-atlas-pipeline.md) — deterministic transformations, atomic export, Sprite Sheet Studio, Atlas Studio, Mirror Targets, and locked generation contracts.

Finish each plan's acceptance gate before starting the next plan. The product design remains the source of truth: [Workmen Design](../specs/2026-07-13-workmen-design.md).
