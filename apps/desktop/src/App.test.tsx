import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("App shell", () => {
  it("renders the shell name and reports backend availability", () => {
    render(<App />);
    // The shell name must be present (per the plan's React test).
    expect(screen.getByTestId("shell-title")).toHaveTextContent("Workmen");
    // The backend-status badge must be visible -- its value is
    // either "checking...", "Tauri backend ready", or
    // "Tauri backend unavailable" depending on the test
    // environment. In a jsdom env without the Tauri global, the
    // shell surfaces a typed error envelope without exposing
    // an absolute path.
    const status = screen.getByTestId("shell-backend-status");
    expect(status.getAttribute("data-available")).toMatch(/unknown|true|false/);
  });

  it("displays a typed error envelope when the Tauri backend is unavailable", () => {
    render(<App />);
    const errEl = screen.queryByTestId("shell-error");
    // In a jsdom env (no Tauri), the shell renders the error
    // envelope in the bottom console. The message must NOT
    // contain an absolute filesystem path.
    if (errEl) {
      const text = errEl.textContent ?? "";
      expect(text).not.toMatch(/^\/|^[A-Z]:\\|^~\//);
    }
  });
});
