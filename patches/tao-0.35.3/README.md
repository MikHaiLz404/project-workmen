# vendored tao 0.35.3 with macOS Sequoia panic mitigation

This is a vendored copy of tao 0.35.3 from
`~/.cargo/registry/src/.../tao-0.35.3/`, modified to catch potential
panics inside the `NSApplicationDidFinishLaunching` callback.

## Why

macOS Sequoia (26.x) on Apple Silicon occasionally delivers the
`applicationDidFinishLaunching:` notification in a state where
`MainThreadMarker::new()` or `apply_activation_policy()` panics.
Because `did_finish_launching` is an `extern "C"` callback,
panics cannot unwind — they trigger `panic_cannot_unwind` →
SIGABRT before the React webview can render.

## What the patch does

Wraps the body of `AppState::launched` (the only function called
from `did_finish_launching`) in `std::panic::catch_unwind`. Panics
are logged to stderr instead of aborting the process. The
worst-case behavior is that the window does not get focus
(which is recoverable) instead of the process dying.

## How Cargo finds it

`[patch.crates-io]` in workspace `Cargo.toml` redirects the
`tao` dependency to this directory.
