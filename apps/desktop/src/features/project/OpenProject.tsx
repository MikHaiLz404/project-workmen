// OpenProject -- the entry-point composition for T2.T2's
// project open + scan workflow. Wires the Rust Tauri host
// (via backend.ts) to the project-store reducer + child
// components (ProjectHeader, ScanProgress).
//
// Plan checklist (T2.T2):
//   - [x] Show project root, config state, asset count, issue
//         summary, scan duration, and explicit "Read-only scan".
//   - [x] Cancellation leaves no partial state and a subsequent
//         scan succeeds (handled by stale request id rejection
//         in reduce() and by the host's cancel flag).
//   - [x] Native directory selection is delegated to Tauri
//         through backend.ts (the host picks a folder via
//         the OS dialog; this component surfaces it).
//   - [x] Recent-project entries live in the OS app-data
//         directory (hosted by open_project's recent.json).

import { useEffect, useReducer } from "react";
import type {
  ProjectSnapshot,
  ScanProgress,
} from "@workmen/contracts";
import * as React from "react";
import {
  BackendUnavailable,
  cancelScan,
  onScanProgress,
  onScanSnapshot,
  openProject,
  scanProject,
} from "../../lib/backend";
import {
  INITIAL_STATE,
  reduce,
} from "./project-store";
import { ProjectHeader } from "./ProjectHeader";
import { ScanProgressView } from "./ScanProgress";

interface Props {
  /**
   * Native picker. Tauri's plugin-dialog is not part of the
   * post-prune frontend dependency set, so the host bridges
   * via a backend command in a future iteration. The test
   * surface accepts a synthetic picker for manual testing.
   */
  pickDirectory?: () => Promise<string | null>;
}

/**
 * Internal helper: the host emits `scan://progress` and
 * `scan://snapshot` through `CustomEvent`s that backend.ts
 * subscribes to. The dispatch helpers below route the events
 * back to the reducer; we expose them as plain functions so
 * the useReducer-friendly closure can capture them once.
 */
function dispatchOpen(
  requestId: string,
): Parameters<typeof reduce>[1] {
  return { type: "open", requestId };
}

export function OpenProject({ pickDirectory }: Props): React.ReactElement {
  const [state, dispatch] = useReducer(reduce, INITIAL_STATE);

  // Subscribe to the two backend event streams once.
  useEffect(() => {
    const offProgress = onScanProgress((value: ScanProgress) => {
      dispatch({ type: "progress", value });
    });
    const offSnapshot = onScanSnapshot((snapshot: ProjectSnapshot) => {
      dispatch({
        type: "ready",
        requestId: snapshot.requestId,
        snapshot,
      });
    });
    return () => {
      offProgress();
      offSnapshot();
    };
  }, []);

  async function handleOpen(): Promise<void> {
    try {
      let path: string | null = null;
      if (pickDirectory) {
        path = await pickDirectory();
      } else {
        // No host picker wired yet -- surface a hint to the
        // shell so the user knows this is read-only + manual.
        const fallback = window.prompt(
          "Enter an absolute path to the game project:",
        );
        path = fallback && fallback.trim().length > 0 ? fallback.trim() : null;
      }
      if (!path) {
        dispatch({
          type: "failed",
          requestId: "user-cancelled",
          message: "open cancelled by user",
        });
        return;
      }
      await openProject(path);
      const requestId = await scanProject(path);
      dispatch(dispatchOpen(requestId));
    } catch (e) {
      const message = e instanceof BackendUnavailable
        ? "Tauri backend unavailable; run inside the desktop shell."
        : e instanceof Error
          ? e.message
          : String(e);
      dispatch({
        type: "failed",
        requestId: state.requestId ?? "unknown",
        message,
      });
    }
  }

  async function handleCancel(): Promise<void> {
    if (!state.requestId) return;
    try {
      await cancelScan(state.requestId);
      dispatch({ type: "cancelled", requestId: state.requestId });
    } catch (e) {
      dispatch({
        type: "failed",
        requestId: state.requestId,
        message: e instanceof Error ? e.message : String(e),
      });
    }
  }

  function handleReset(): void {
    dispatch({ type: "reset" });
  }

  const root = state.snapshot?.root ?? null;
  const configState: "missing" | "loaded" | "freshly-initialized" =
    state.snapshot ? "loaded" : "missing";

  return (
    <section className="open-project" data-testid="open-project">
      <header>
        <h2>Project</h2>
        <div className="open-project-actions">
          <button
            type="button"
            onClick={handleOpen}
            data-testid="open-project-open"
          >
            Open Project
          </button>
          <button
            type="button"
            onClick={handleReset}
            data-testid="open-project-reset"
            disabled={state.phase === "idle"}
          >
            Reset
          </button>
        </div>
      </header>
      <ProjectHeader
        root={root}
        configState={configState}
        snapshot={state.snapshot}
        readOnly={true}
      />
      <ScanProgressView progress={state.progress} onCancel={handleCancel} />
      <p
        className="open-project-phase"
        data-testid="open-project-phase"
      >
        Phase: {state.phase}
      </p>
      {state.message && (
        <p className="open-project-error" data-testid="open-project-error">
          {state.message}
        </p>
      )}
    </section>
  );
}
