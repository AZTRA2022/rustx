//! What Linux can do, and how.
//!
//! The same files as `windows/`, with the same function names, so that
//! what is above this directory never asks which platform it is on.
//! `clipboard.rs`, `notify.rs`, `autostart.rs`, `instance.rs`,
//! `dialog.rs` and the camera list in `devices.rs` are real — each by
//! a program or a file every desktop distribution already carries.
//! `shortcut.rs`, `tray.rs` and `window.rs` still refuse, each naming
//! what has to be written and why it is not a file-at-a-time job (a
//! desktop portal over D-Bus, or a connection to the display server
//! this crate does not hold).

pub mod autostart;
pub mod clipboard;
pub mod devices;
pub mod dialog;
pub mod instance;
pub mod notify;
pub mod shortcut;
pub mod tray;
pub mod window;
