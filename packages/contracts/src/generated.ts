// AUTO-GENERATED FILE -- do not edit by hand.
// Source of truth: crates/workmen-core/src/model (Asset, Profile,
// ValidationIssue) and crates/workmen-core/src/bindings.rs
// (CommandEnvelope, WorkmenBackend).
//
// Regenerate with: `cargo run -p workmen-core --example
// emit_typescript_contracts` (T2.T1 follow-up). The drift gate in
// crates/workmen-core/tests/typed_contracts.rs fails when this file
// is stale.

export interface CommandEnvelope<T> {
  apiVersion: 1;
  requestId: string;
  data: T;
}

export interface CommandError {
  code: string;
  message: string;
}

export interface WorkmenBackend {
  invoke<TRequest, TResponse>(
    command: string,
    request: TRequest,
  ): Promise<CommandEnvelope<TResponse> | { apiVersion: 1; requestId: string; error: CommandError }>;
}

// Core model types -- mirrors crates/workmen-core/src/model/asset.rs,
// profile.rs, and validation.rs. Field names follow Rust's
// `#[serde(rename_all = "camelCase")]` convention.
export interface Asset {
  id: string;
  path: string;
  role: 'source' | 'runtime' | 'derived' | 'mirrorTarget' | 'excluded' | 'unclassified';
  format:
    | 'png'
    | 'jpg'
    | 'webP'
    | 'svg'
    | 'iosAssetCatalogJson'
    | 'androidVectorXml'
    | 'androidAdaptiveIconXml'
    | 'runtimeManifestJs'
    | { other: string };
  metadata:
    | { kind: 'raster'; width: number; height: number; encodedBytes: number; decodedBytes: number; hasAlpha: boolean; colorType: string; bitDepth: number; alphaBounds?: Rect }
    | { kind: 'vector'; viewBox?: ViewBox; rasterPreviewTargets: PixelSize[] };
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ViewBox {
  minX: number;
  minY: number;
  width: number;
  height: number;
}

export interface PixelSize {
  width: number;
  height: number;
}

export interface Profile {
  schemaVersion: number;
  id: string;
  profileRevision: number;
  state: 'draft' | 'active' | 'locked';
  matchers: ProfileMatcher[];
  namingRules: NamingRule[];
  sourceRuntime: SourceRuntimeRelationship[];
  exceptions: ProfileException[];
  budgets: PlatformBudget[];
}

export interface ProfileMatcher {
  pathGlob?: string;
  namingPattern?: string;
  extension?: string;
  assetRole?: 'source' | 'runtime' | 'derived' | 'mirrorTarget' | 'excluded' | 'unclassified';
}

export interface NamingRule {
  pattern: string;
  description: string;
}

export interface SourceRuntimeRelationship {
  source: string;
  runtime: string;
}

export interface ProfileException {
  ruleId: string;
  assetMatcher: ProfileMatcher;
  reason: string;
  expiresAt?: string;
}

export interface PlatformBudget {
  platform: 'web' | 'ios' | 'android';
  maxTextureWidth: number;
  maxTextureHeight: number;
  maxEncodedBytes: number;
  maxDecodedBytes: number;
  colorSpace: string;
}

export interface ProjectSnapshot {
  requestId: string;
  root: string;
  files: Array<{
    /** File path relative to the project root. */
    path: string;
    /** Stable, deterministic identity for the asset. */
    id: string;
    role:
      | "source"
      | "runtime"
      | "derived"
      | "mirrorTarget"
      | "excluded"
      | "unclassified";
    format: string;
    encodedBytes: number;
    blake3?: string;
  }>;
  diagnostics: Array<{
    path: string;
    kind:
      | "decodeError"
      | "ioError"
      | "symlinkSkipped"
      | "excluded"
      | "unsupportedFormat";
    message: string;
  }>;
  durationMs: number;
}

export interface ScanProgress {
  requestId: string;
  phase: "opening" | "scanning" | "ready" | "failed" | "cancelled";
  completed: number;
  total: number | null;
  relativePath: string | null;
}

export interface ProjectPhase {
  phase: "idle" | "opening" | "scanning" | "ready" | "failed" | "cancelled";
}

export interface ProjectAction {
  type:
    | { kind: "open"; requestId: string }
    | { kind: "progress"; value: ScanProgress }
    | { kind: "ready"; requestId: string; snapshot: ProjectSnapshot }
    | { kind: "failed"; requestId: string; message: string };
}

export interface ValidationIssue {
  assetPath: string;
  diff: SpecDiff;
}

export interface SpecDiff {
  ruleId: string;
  profileId: string;
  expected: unknown;
  actual: unknown;
  platform?: 'web' | 'ios' | 'android';
  severity: 'error' | 'warning' | 'info';
  suggestedAction: string;
}
