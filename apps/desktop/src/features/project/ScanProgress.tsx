// ScanProgress -- live progress bar driven by the store's
// latest ScanProgress payload. Includes a Cancel button that
// calls back through the supplied onCancel handler, which the
// OpenProject component wires to the Tauri backend.

import type { ScanProgress } from "@workmen/contracts";
import * as React from "react";

interface Props {
  progress: ScanProgress | null;
  onCancel: () => void;
}

export function ScanProgressView({
  progress,
  onCancel,
}: Props): React.ReactElement {
  if (!progress) return <></>;
  const total = progress.total;
  const percent = total && total > 0
    ? Math.min(100, Math.round((progress.completed / total) * 100))
    : 0;
  return (
    <div className="scan-progress" data-testid="scan-progress">
      <div className="scan-progress-row">
        <span className="scan-progress-phase" data-testid="scan-progress-phase">
          {progress.phase}
        </span>
        <span className="scan-progress-count" data-testid="scan-progress-count">
          {progress.completed}
          {total !== null ? ` / ${total}` : ""}
        </span>
      </div>
      <div
        className="scan-progress-bar"
        role="progressbar"
        aria-valuenow={percent}
        aria-valuemin={0}
        aria-valuemax={100}
        data-testid="scan-progress-bar"
      >
        <div
          className="scan-progress-bar-fill"
          style={{ width: `${percent}%` }}
        />
      </div>
      {progress.relativePath && (
        <div
          className="scan-progress-current"
          data-testid="scan-progress-current"
        >
          {progress.relativePath}
        </div>
      )}
      <button
        type="button"
        className="scan-progress-cancel"
        onClick={onCancel}
        data-testid="scan-progress-cancel"
      >
        Cancel
      </button>
    </div>
  );
}
