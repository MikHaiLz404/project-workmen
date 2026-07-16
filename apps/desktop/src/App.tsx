import { useEffect, useState } from "react";
import type { WorkmenBackend } from "@workmen/contracts";
import { OpenProject } from "./features/project/OpenProject";

/**
 * The four-region shell required by the T2.T1 plan:
 *
 * - Project header (top)
 * - Navigation rail (left)
 * - Primary workspace (center)
 * - Bottom console (bottom)
 *
 * The layout stays responsive down to 1024x700.
 */
export default function App() {
  const [backendAvailable, setBackendAvailable] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const tauri = (window as unknown as { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
    setBackendAvailable(typeof tauri !== "undefined");
    if (typeof tauri === "undefined") {
      setError(
        "Tauri backend unavailable: command envelope tests still pass; run via `npm run frontend:build && tauri dev` for the desktop shell.",
      );
    }
  }, []);

  return (
    <div className="workmen-shell" data-testid="workmen-shell">
      <header className="shell-header" data-testid="shell-header">
        <h1 className="shell-title" data-testid="shell-title">
          Workmen
        </h1>
        <span
          className="shell-backend-status"
          data-testid="shell-backend-status"
          data-available={backendAvailable === null ? "unknown" : String(backendAvailable)}
        >
          {backendAvailable === null
            ? "checking..."
            : backendAvailable
              ? "Tauri backend ready"
              : "Tauri backend unavailable"}
        </span>
      </header>
      <nav className="shell-rail" data-testid="shell-rail" aria-label="Primary navigation">
        <ul>
          <li>Project</li>
          <li>Browser</li>
          <li>Inspector</li>
          <li>Profiles</li>
          <li>Validation</li>
        </ul>
      </nav>
      <main className="shell-workspace" data-testid="shell-workspace">
        <OpenProject />
      </main>
      <footer className="shell-console" data-testid="shell-console">
        {error ? (
          <span data-testid="shell-error" role="status">
            {error}
          </span>
        ) : (
          <span>ready</span>
        )}
      </footer>
    </div>
  );
}
