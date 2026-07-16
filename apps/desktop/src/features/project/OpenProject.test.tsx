// Manual demo reference for the OpenProject component.
//
// Like project-store.test.ts there is no JS test runner after
// the dev-tooling prune. This file is exercised at build time
// only (tsc --noEmit). Manual clicks in the Tauri runtime are
// the source of truth for behavior.

import { OpenProject } from "./OpenProject";

export function _openProjectSpec(): typeof OpenProject {
  // Reference the component so tsc keeps the module compiled
  // alongside its sibling store.
  void OpenProject;
  return OpenProject;
}
