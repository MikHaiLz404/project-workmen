# Workmen Context

Shared product language for Workmen, a standalone game-asset workbench. Workmen inspects game projects, records asset contracts, validates actual files against those contracts, and produces derived outputs without changing source art.

## Product Structure

**Workbench Module**:
A major workspace that groups related asset work, such as project scanning, profile authoring, spritesheet preparation, or atlas assembly.
_Avoid_: Tool, command, panel

**Tool**:
A focused operation inside a Workbench Module, such as Slice, Trim, Compare, or Pack.
_Avoid_: Module, core service

**Game Project**:
A directory tree containing a game's source art, runtime art, manifests, and platform-specific asset bundles.
_Avoid_: Workmen project, art folder

## Asset Model

**Asset**:
A game-art file or an art-specific metadata file recognized by Workmen.
_Avoid_: Every file in the repository

**Asset Family**:
A group of Assets that share one purpose and contract, such as item icons, portraits, backdrops, or VFX frames.
_Avoid_: Folder, file extension

**Asset Role**:
The relationship an Asset has to the production pipeline: Source, Runtime, Derived, Mirror Target, Excluded, or Unclassified.
_Avoid_: Asset type, validation status

**Source Asset**:
The authoritative art input preserved without in-place modification by Workmen.
_Avoid_: Runtime asset, editable output

**Runtime Asset**:
An Asset intended to be consumed directly by the game at runtime.
_Avoid_: Source asset, build mirror

**Derived Asset**:
An Asset produced from one or more Source Assets by an explicit operation or export recipe.
_Avoid_: Source asset, temporary preview

**Mirror Target**:
A generated shipping copy whose content must match its Runtime Asset but is not counted as a separate inventory item.
_Avoid_: Runtime source, duplicate asset

**Excluded Asset**:
An Asset discovered under an ignored, deprecated, rejected, or generated-cache path and omitted from shipping pass/fail results.
_Avoid_: Deleted asset, missing asset

**Unclassified Asset**:
A discovered Asset that cannot be assigned confidently to one Asset Family or Profile.
_Avoid_: Invalid asset, excluded asset

## Contracts

**Preset**:
A reusable and shareable starting contract from which a project Profile can be created.
_Avoid_: Live project policy, profile update

**Profile**:
A version-controlled contract that binds an Asset Family to matchers, visual constraints, platform budgets, and export rules in one Game Project.
_Avoid_: Preset, global default

**Locked Profile**:
An active Profile whose revision must be explicitly unlocked before its contract can change.
_Avoid_: Read-only file, immutable preset

**Naming Matcher**:
A rule that assigns Assets using paths and naming conventions without implying that their image content is invalid.
_Avoid_: Image validator, filename fixer

**Platform Budget**:
A Profile's target-specific limits for texture dimensions, encoded size, decoded memory, color handling, alpha, and compression.
_Avoid_: Art direction, gameplay budget

**Spec Diff**:
The structured difference between an actual Asset and its resolved Profile, expressed as expected and actual values.
_Avoid_: Git diff, change history

**Validation Issue**:
A Spec Diff evaluated by a validation rule and assigned Error, Warning, or Info severity.
_Avoid_: Operation log, raw exception

**Layout Lock**:
An atlas contract that preserves existing sprite slots or rectangles until a user explicitly approves a full repack.
_Avoid_: Frozen image, uneditable atlas

**Generator Pack**:
A provider-neutral bundle containing a Locked Profile snapshot, technical brief, expected outputs, naming map, and visual guides for external asset generation.
_Avoid_: Generator integration, generated asset
