import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";

// Styles are inlined in index.html -- see <style> tag.
// We import nothing from "./styles.css" because tsc does not
// bundle CSS; the browser receives the same rules via index.html.

const container = document.getElementById("root");
if (!container) {
  throw new Error("workmen: #root element not found");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
