# Tauri 0.35.3 panics in `did_finish_launching` on macOS Sequoia 26.5.x

## Status

**Mitigated by Path C** (vendored `tao` patch). Commit `d6e4b3d` (see git log) adds `patches/tao-0.35.3/` and a `[patch.crates-io] tao = { path = ... }` redirect in workspace `Cargo.toml`. The patch wraps `AppState::launched` (the only function called from `did_finish_launching`) in `std::panic::catch_unwind`; a panic from `apply_activation_policy` or `MainThreadMarker::new().unwrap()` is logged to stderr instead of unwinding through the FFI callback. `target/debug/workmen-desktop` now stays alive past the 3-second crash window on the user's macOS Sequoia 26.5.2 box.

## Context

M2 (Desktop Workbench, issue #2) was scoped after M1 to add a Tauri 2 desktop shell over the `workmen-cli` binary. We scaffold the shell with `apps/desktop/src-tauri/` and ship the typed contracts (`packages/contracts/src/generated.ts`), the React state machine (`apps/desktop/src/features/project/`), and the wire-to-Rust bridge (`apps/desktop/src/lib/backend.ts`).

While validating on the user's machine — macOS 26.5.2 (Apple M-series), Tauri 2.11.5, tao 0.35.3 — every launch of `target/release/workmen-desktop` (or a debug build via `cargo run -p workmen-desktop`) resulted in `Abort trap: 6`. The Mach-O `__TEXT` segment is `app_delegate::did_finish_launching` in `tao-0.35.3/src/platform_impl/macos/app_delegate.rs:125` calling `AppState::launched(this)` (`app_state.rs:284`). Inside that function, a `panic_cannot_unwind` fires while the AppKit `NSApplicationDidFinishLaunchingNotification` is dispatched on the main thread — exactly the FFI context where Rust cannot unwind. The process dies before `WindowEventLoop::run` can dispatch any event.

Reproduced three times with three independent invocations (`target/release/workmen-desktop`, `tauri dev`, `target/debug/workmen-desktop`). Crash reports persist under `~/Library/Logs/DiagnosticReports/workmen-desktop-*.ips`.

This is upstream: the panic originates inside `tao 0.35.3`. The Tauri, tauri-runtime, wry, and tauri-build toolchain at 2.11.x is forced to that `tao`. macOS Sequoia 26.5.x introduced notification-delivery timing where `MainThreadMarker::new().unwrap()` or the subsequent AppKit calls panic under `panic_cannot_unwind` rather than returning a recoverable error. The exact offending call site is at the boundary between Apple's `-[NSApp _postDidFinishNotification:] → __CFNOTIFICATIONCENTER_IS_CALLING_OUT_TO_AN_OBSERVER__ → did_finish_launching` — Taury's `window_activation_hack` and `apply_activation_policy` blocks emit the panic. We are not the only maintainer of that code path: the call originates inside the `tauri::Builder::run` invocation in `apps/desktop/src-tauri/src/lib.rs:35` (`tauri::generate_context!` embedded assets are intact — the panic is downstream of asset decode).

Workarounds attempted on this machine:

- Manual signing via `codesign --force --deep --sign -` plus `xattr -dr com.apple.quarantine`. No change; same panic.
- A short `Path B` experiment (now reverted) which pinned `tauri = "=2.7.0"`, `tauri-build = "=2.3.1"`, `tauri-runtime-wry = "=2.7.2"`, attempted to lock the toolchain to `tao 0.33.x`. The cross-crate version drift between `tauri`, `tauri-build`, `tauri-runtime-wry`, and `tao` causes every stable combo we tried to break compile (window-dispatcher trait mismatch `E0046`; tauri-utils signature change `E0061`). We reverted via `git checkout HEAD`; tree returned to `fec8338`.
- `Path B` retry on a Linux/Windows runner is out of scope: the user runs Workmen on macOS Sequoia, so the macOS path is the blocker.

## Decision

For this milestone, the canonical UI is the **Tauri 2 desktop shell patched to swallow the macOS Sequoia launch panic**, alongside the **Rust CLI** as a fallback. The patch lives at `patches/tao-0.35.3/` and is wired into the workspace via `[patch.crates-io] tao = { path = "patches/tao-0.35.3" }`. The patch is a local divergence from `tao-v0.35.3` upstream: only `apps/desktop/...` builds consume the patched path; tauri-cli and the M1 Rust core are unaffected.

The CLI is still useful as a non-graphical fallback; the M1 acceptance gate already verified the CLI on `/Users/jojo/Github/project-merge-ios`: 466 files scanned, 0 diagnostics, exit 0 (`8f81260`).

The Tauri scaffold (`apps/desktop/src-tauri/`), the React state machine (`apps/desktop/src/features/project/`), the typed contracts (`packages/contracts/src/generated.ts`), and the wire bridge (`apps/desktop/src/lib/backend.ts`) remain in the tree as reference. They compile and lint clean.

## Consequences

- The patch is local and reversible: removing `patches/tao-0.35.3/` plus the `[patch.crates-io]` block restores the pure upstream `tao` resolution. We expect to remove the patch when:
  1. Upstream `tao` ships a fix (next minor release of `tao` after 0.35.3), **or**
  2. The user migrates to a different frontend host (e.g., a future Tauri that no longer calls `apply_activation_policy` synchronously inside `did_finish_launching`).
- Issues #2 (M2 epic) and #13 (T2.T2) remain in their current state. T2.T2's Rust-side commands are committed at `b4ff5b8`; T2.T2's React shell at `fec8338` depends on the patched Tauri runtime to actually render the webview.
- Future M2-tasks (T2.T3 Asset Browser, T2.T4 Inspector, T2.T5 Profile Studio, T2.T6 Validation Console, T2.T7 release gates) remain plan-relevant. Their frontend components can now be validated in the running Tauri window; their Rust sides can be validated via `cargo test`.

## Alternatives considered

- **Path B (re-attempt downgrade)** — too much cross-crate drift per attempt; risk of another revert cycle. Reject.
- **Drop Tauri from M2 entirely (rewrite M2 to a Taur/Cursive/Crossterm TUI)** — violates the user's "ทำ desktop app" requirement. Reject.
- **Wait for upstream fix only** — would have blocked this milestone for an unbounded time; the user already chose Path C / Path A depending on context.
