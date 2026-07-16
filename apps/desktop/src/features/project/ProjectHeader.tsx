// ProjectHeader -- read-only header showing the resolved
// project root, config state, and asset count. The plan calls
// for an explicit "Read-only scan" indicator so the user knows
// the scan never mutates the game project.

import type { ProjectSnapshot } from "@workmen/contracts";
import * as React from "react";

interface Props {
  root: string | null;
  configState: "missing" | "loaded" | "freshly-initialized";
  snapshot: ProjectSnapshot | null;
  readOnly: boolean;
}

export function ProjectHeader({
  root,
  configState,
  snapshot,
  readOnly,
}: Props): React.ReactElement {
  return (
    <header className="project-header" data-testid="project-header">
      <div className="project-header-row">
        <span className="project-header-label">Project root</span>
        <code className="project-header-value" data-testid="project-root">
          {root ?? "(none)"}
        </code>
      </div>
      <div className="project-header-row">
        <span className="project-header-label">Config</span>
        <span className="project-header-value" data-testid="project-config">
          {configState}
        </span>
      </div>
      <div className="project-header-row">
        <span className="project-header-label">Assets</span>
        <span className="project-header-value" data-testid="project-assets">
          {snapshot ? snapshot.files.length : 0}
        </span>
      </div>
      <div className="project-header-row">
        <span className="project-header-label">Diagnostics</span>
        <span className="project-header-value" data-testid="project-diagnostics">
          {snapshot ? snapshot.diagnostics.length : 0}
        </span>
      </div>
      {snapshot && (
        <div className="project-header-row">
          <span className="project-header-label">Duration</span>
          <span className="project-header-value" data-testid="project-duration">
            {snapshot.durationMs} ms
          </span>
        </div>
      )}
      <div
        className="project-header-readonly"
        data-testid="project-readonly-badge"
        aria-label={readOnly ? "Scan is read-only" : undefined}
      >
        {readOnly ? "Read-only scan" : ""}
      </div>
    </header>
  );
}
