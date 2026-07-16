// Workmen backend bridge.
//
// Tauri exposes the `__TAURI_INTERNALS__` global on the window
// once the app boots. Every command invocation and event
// subscription goes through this module so the React side
// never touches the global directly. (Plan T2.T1 contract:
// "command envelope { apiVersion, requestId, data | error }".
// Tauri 2 returns `{ data, error }` shaped payloads that we
// normalise here.)

import type {
  ProjectSnapshot,
  ScanProgress,
} from "@workmen/contracts";

interface TauriInternals {
  invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
}

function tauri(): TauriInternals | null {
  if (typeof window === "undefined") return null;
  const internal = (window as unknown as {
    __TAURI_INTERNALS__?: TauriInternals;
  }).__TAURI_INTERNALS__;
  return internal ?? null;
}

export class BackendUnavailable extends Error {
  constructor() {
    super("Tauri backend unavailable: run via the desktop shell.");
    this.name = "BackendUnavailable";
  }
}

function requireBackend(): TauriInternals {
  const t = tauri();
  if (!t) throw new BackendUnavailable();
  return t;
}

export async function openProject(
  path: string,
): Promise<{ root: string; marker: "git" | "workmen" | "supplied" }> {
  const t = requireBackend();
  type RootInfo = {
    path: { toString(): string };
    marker: "GitDir" | "WorkmenDir" | "SuppliedDir";
  };
  const root = await t.invoke<RootInfo>("open_project", { path });
  return {
    root: root.path.toString(),
    marker:
      root.marker === "GitDir"
        ? "git"
        : root.marker === "WorkmenDir"
          ? "workmen"
          : "supplied",
  };
}

export async function scanProject(path: string): Promise<string> {
  const t = requireBackend();
  return await t.invoke<string>("scan_project", { path });
}

export async function cancelScan(requestId: string): Promise<void> {
  const t = requireBackend();
  await t.invoke<void>("cancel_scan", { requestId });
}

// Tauri 2 forwards backend events to the webview as
// CustomEvents named "workmen://scan-progress" and
// "workmen://scan-snapshot" (see project.rs App::emit sites).
// We surface thin subscriptions here so the React side
// never touches `window.addEventListener` directly.

export function onScanProgress(
  cb: (progress: ScanProgress) => void,
): () => void {
  const handler = (e: Event): void => {
    const ce = e as CustomEvent<ScanProgress>;
    cb(ce.detail);
  };
  window.addEventListener("workmen://scan-progress", handler);
  return () =>
    window.removeEventListener("workmen://scan-progress", handler);
}

export function onScanSnapshot(
  cb: (snapshot: ProjectSnapshot) => void,
): () => void {
  const handler = (e: Event): void => {
    const ce = e as CustomEvent<ProjectSnapshot>;
    cb(ce.detail);
  };
  window.addEventListener("workmen://scan-snapshot", handler);
  return () =>
    window.removeEventListener("workmen://scan-snapshot", handler);
}
