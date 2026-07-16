// Manual demo / spec reference for the project-store reducer.
//
// After the dev-tooling prune (commit 47e8ba7) there is no JS
// test runner. This file documents the reducer's contract for
// reviewers and acts as a runnable type check at build time
// (the `tsc --noEmit` gate validates that the reduce() inputs
// and outputs stay in sync with the plan spec).
//
// To exercise these cases manually:
//   npm run desktop:dev
// then drive the OpenProject workflow with the Tauri runtime.

import {
  INITIAL_STATE,
  reduce,
} from "./project-store";

export function _manualDemo(): void {
  // idle -> opening on "open"
  const a = reduce(INITIAL_STATE, { type: "open", requestId: "scan-1" });
  if (a.phase !== "opening") throw new Error("open should set phase=opening");

  // opening -> scanning on first progress
  const b = reduce(a, {
    type: "progress",
    value: {
      requestId: "scan-1",
      phase: "scanning",
      completed: 0,
      total: 10,
      relativePath: null,
    },
  });
  if (b.phase !== "scanning") throw new Error("scanning progress should set phase=scanning");

  // stale progress is dropped
  const stale = reduce(b, {
    type: "progress",
    value: {
      requestId: "scan-OTHER",
      phase: "scanning",
      completed: 99,
      total: 10,
      relativePath: null,
    },
  });
  if (stale !== b) throw new Error("stale request id must be a no-op");

  // scanning -> ready on snapshot
  const c = reduce(b, {
    type: "ready",
    requestId: "scan-1",
    snapshot: {
      requestId: "scan-1",
      root: "/tmp/game",
      files: [],
      diagnostics: [],
      durationMs: 100,
    },
  });
  if (c.phase !== "ready") throw new Error("ready must set phase=ready");

  // ready -> cancelled preserves stale-id rejection
  const c2 = reduce(c, { type: "cancelled", requestId: "scan-OTHER" });
  if (c2 !== c) throw new Error("cancelled with stale id must be a no-op");

  // reset clears state
  const d = reduce(c, { type: "reset" });
  if (d !== INITIAL_STATE) throw new Error("reset must equal INITIAL_STATE");
}
