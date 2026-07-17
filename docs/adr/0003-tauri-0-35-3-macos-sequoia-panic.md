# Tauri 0.35.3 panics in `did_finish_launching` on macOS Sequoia 26.5.x

## Status

Accepted (working around).

## Context

M2 (Desktop Workbench, issue #2) was scoped after M1 to add a Tauri 2 desktop shell over the `workmen-cli` binary. We scaffold the shell with `apps/desktop/src-tauri/` and ship the typed contracts (`packages/contracts/src/generated.ts`), the React state machine (`apps/desktop/src/features/project/`), and the wire-to-Rust bridge (`apps/desktop/src/lib/backend.ts`).

While validating on the user's machine — macOS 26.5.2 (Apple M-series), Tauri 2.11.5, tao 0.35.3 — every launch of `target/release/workmen-desktop` (or a debug build via `cargo run -p workmen-desktop`) resulted in `Abort trap: 6`. The Mach-O `__TEXT` segment is `app_delegate::did_finish_launching` in `tao-0.35.3/src/platform_impl/macos/app_delegate.rs:125` calling `AppState::launched(this)` (`app_state.rs:284`). Inside that function, a `panic_cannot_unwind` fires while the AppKit `NSApplicationDidFinishLaunchingNotification` is dispatched on the main thread — exactly the FFI context where Rust cannot unwind. The process dies before `WindowEventLoop::run` can dispatch any event.

Reproduced three times with three independent invocations (`target/release/workmen-desktop`, `tauri dev`, `target/debug/workmen-desktop`). Crash reports persist under `~/Library/Logs/DiagnosticReports/workmen-desktop-*.ips`.

This is upstream: the panic originates inside `tao 0.35.3`. The Tauri, tauri-runtime, wry, and tauri-build toolchain at 2.11.x is forced to that `tao`. macOS Sequoia 26.5.x introduced notification-delivery timing where `MainThreadMarker::new().unwrap()` or the subsequent AppKit calls panic under `panic_cannot_unwind` rather than returning a recoverable error. The exact offending call site is at the boundary between Apple's `-[NSApp _postDidFinishNotification:] → __CFNOTIFICATIONCENTER_IS_CALLING_OUT_TO_AN_OBSERVER__ → did_finish_launching` — Taury's `window_activation_hack` and `apply_activation_policy` blocks emit the panic. We are not the only maintainer of that code path: the call originates inside the `tauri::Builder::run` invocation in `apps/desktop/src-tauri/src/lib.rs:35` (`tauri::generate_context!` embedded assets are intact — the panic is downstream of asset decode).

Workarounds attempted on this machine:

- Manual signing via `codesign --force --deep --sign -` plus `xattr -dr com.apple.quarantine`. No change; same panic.
- A short `Path B` experiment (now reverted) which pinned `tauri = "=2.7.0"`, `tauri-build = "=2.3.1"`, `tauri-runtime-wry = "=2.7.2"`, attempted to lock the toolchain to `tao 0.33.x`. The cross-crate version drift between `tauri`, `tauri-build`, `tauri-runtime-wry`, and `tao` causes every stable combo we tried to break compile (window-dispatcher trait mismatch `E0046`; tauri-utils signature change `E0061`). We reverted via `git checkout HEAD`; tree returns to `fec8338`.
- `Path C` (vendor-patch `tao`) and a `Path B` retry on a Linux/Windows runner are out of scope for this milestone. The user runs Workmen on macOS Sequoia, so the macOS path is the blocker; the others do not consume engineering time here.

## Decision

For this milestone, the canonical UI is the **Rust CLI** (`crates/workmen-cli/`), not the Tauri desktop shell. The M1 acceptance gate already verified the CLI on `/Users/jojo/Github/project-merge-ios`: 466 files scanned, 0 diagnostics, exit 0 (`8f81260`).

The Tauri scaffold (`apps/desktop/src-tauri/`) and the React shell (`apps/desktop/src/`) remain in the tree as reference. They compile, lint, and pass the typed-contracts drift gate. They are runnable on any machine whose Tauri stack does not panic in `did_finish_launching`. We explicitly do not claim that `target/release/workmen-desktop` is a runnable binary on the user's machine today.

When the upstream fix lands (one of: a `tao` release that handles Sequoia 26.5+ notification timing, a local fork, or a `Path C` vendor-patch), the existing scaffold compiles unchanged and the typed contracts carry over.

## Consequences

- M2 deliverables and their verification: the verifier at `/var/folders/4m/.../T/hermes-verify-fec8338.sh` asserts (a) HEAD is `fec8338` post-`47e8ba7` (CVE-prune), (b) no Path B pins remain (`[patch.crates-io]` absent; `tauri = { workspace = true }` in `apps/desktop/src-tauri/Cargo.toml`), (c) all Rust + npm gates green, (d) `workmen scan /Users/jojo/Github/project-merge-ios` returns `466 files, 0 diagnostics, exit 0`, (e) `npm audit` reports 0 vulnerabilities, (f) `local HEAD == origin/main`.
- Issues #2 (M2 epic) and #13 (T2.T2) are reopened with this scope; #14–#18 carry the same comment. None of them is closed.
- Future M2-tasks (T2.T3 Asset Browser, T2.T4 Inspector, T2.T5 Profile Studio, T2.T6 Validation Console, T2.T7 release gates) remain plan-relevant but cannot be verified end-to-end until the host is healthy.

## Alternatives considered

- **Path B retry** — too much cross-crate drift per attempt; runtime cost unknown.
- **Path C vendor-patch `tao` via `cargo patch` against `tao-v0.35.3` with `catch_unwind` around `apply_activation_policy` and `window_activation_hack`** — would unblock the build at the cost of a maintenance patch in `patches/`. Defer.
- **Drop Tauri from M2 entirely (rewrite M2 to a Taur/Cursive/Crossterm TUI)** — violates the user's "ทำ desktop app" requirement. Reject.
