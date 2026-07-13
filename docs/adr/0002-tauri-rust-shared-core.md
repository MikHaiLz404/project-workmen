# Use Tauri with one Rust core for desktop and CLI

Workmen ships as a cross-platform Tauri desktop application with a React and TypeScript interface, plus a native CLI. Both surfaces call the same Rust core so scanning, image analysis, validation, atlas packing, and reports remain deterministic and identical across GUI and automation. Electron was considered for its uniform bundled Chromium runtime, but the shared native CLI, image-processing workload, and long-term tool-suite direction favor a small native core that does not require bundling Chromium.
