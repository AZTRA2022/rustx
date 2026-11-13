//! What macOS can do, and how.
//!
//! The same files as `windows/` and `linux/`, with the same function
//! names, so that what is above this directory never asks which
//! platform it is on. `clipboard.rs`, `notify.rs`, `autostart.rs`,
//! `instance.rs` and the camera list in `devices.rs` are real, each by
//! a tool or a mechanism macOS already ships: `pbcopy`/`pbpaste`,
//! `osascript`, a `LaunchAgent` plist, a Unix domain socket,
//! `system_profiler`. `dialog.rs` is real too, by the same door —
//! `osascript` saying `choose file`. `shortcut.rs` (Carbon's
//! `RegisterEventHotKey`), `tray.rs` (`NSStatusItem`) and `window.rs`
//! (`NSWindow`) are written, through `objc2` and `appkit.rs` — and
//! have never been compiled for a Mac nor run; only type-checked on
//! Windows against `objc2` built for GNUstep. Each says so at its top.

mod appkit;
pub mod autostart;
pub mod clipboard;
pub mod devices;
pub mod dialog;
pub mod instance;
pub mod notify;
pub mod shortcut;
pub mod tray;
pub mod window;
