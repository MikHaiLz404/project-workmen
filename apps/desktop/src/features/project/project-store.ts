// Project state machine.
//
// The plan calls for a reducer-based state store covering
// idle / opening / scanning / ready / failed / cancelled with
// stale request id rejection. The store consumes two event
// streams from the Tauri backend: scan://progress (intermediate
// phase changes) and scan://snapshot (terminal snapshot).
//
// Without a JS test runner after the prune (vitest removed --
// commit 47e8ba7), this store is exercised manually in the
// running Tauri app. The Rust integration suite under
// apps/desktop/src-tauri/tests/project_integration.rs
// exercises the cancel-registry contract on the host side.

import type {
  ProjectSnapshot,
  ScanProgress,
} from "@workmen/contracts";

export type Phase =
  | "idle"
  | "opening"
  | "scanning"
  | "ready"
  | "failed"
  | "cancelled";

export interface ProjectState {
  phase: Phase;
  /** Currently-tracked scan request id. */
  requestId: string | null;
  /** Last progress update from the backend. */
  progress: ScanProgress | null;
  /** Final snapshot when phase === "ready". */
  snapshot: ProjectSnapshot | null;
  /** Failure message when phase === "failed". */
  message: string | null;
}

export const INITIAL_STATE: ProjectState = {
  phase: "idle",
  requestId: null,
  progress: null,
  snapshot: null,
  message: null,
};

export type Action =
  | { type: "open"; requestId: string }
  | { type: "progress"; value: ScanProgress }
  | { type: "ready"; requestId: string; snapshot: ProjectSnapshot }
  | { type: "failed"; requestId: string; message: string }
  | { type: "cancelled"; requestId: string }
  | { type: "reset" };

/** Reduce the project state. Stale request ids are dropped:
 * if an action arrives carrying an id that does not match the
 * currently-tracked request, the action is a no-op. */
export function reduce(
  state: ProjectState,
  action: Action,
): ProjectState {
  switch (action.type) {
    case "open":
      return {
        phase: "opening",
        requestId: action.requestId,
        progress: null,
        snapshot: null,
        message: null,
      };
    case "progress":
      // Drop stale progress for ids we never tracked or that
      // already finished.
      if (state.requestId !== action.value.requestId) return state;
      // Map the planning's intermediate phases to a coherent
      // phase label. "scanning" dominates; "opening" precedes
      // it; the terminal phases are handled by other actions.
      return {
        ...state,
        progress: action.value,
        phase: action.value.phase === "opening" ? "opening" : "scanning",
      };
    case "ready":
      if (state.requestId !== action.requestId) return state;
      return {
        phase: "ready",
        requestId: action.requestId,
        progress: state.progress,
        snapshot: action.snapshot,
        message: null,
      };
    case "failed":
      if (state.requestId !== action.requestId) return state;
      return {
        phase: "failed",
        requestId: action.requestId,
        progress: state.progress,
        snapshot: null,
        message: action.message,
      };
    case "cancelled":
      if (state.requestId !== action.requestId) return state;
      return {
        phase: "cancelled",
        requestId: action.requestId,
        progress: state.progress,
        snapshot: null,
        message: null,
      };
    case "reset":
      return INITIAL_STATE;
    default: {
      const _exhaustive: never = action;
      return state;
    }
  }
}
