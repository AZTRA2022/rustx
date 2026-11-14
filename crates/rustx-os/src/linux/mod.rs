//! What Linux can do, and how.
//!
//! The same files as `windows/`, with the same function names, so that
//! what is above this directory never asks which platform it is on.
//! `clipboard.rs`, `notify.rs`, `autostart.rs`, `instance.rs`,
//! `dialog.rs` and the camera list in `devices.rs` are real — each by
//! a program or a file every desktop distribution already carries.
//! `window.rs` is real on X11 (and on Wayland through XWayland), by
//! this crate's own connection to the display server — compile-checked
//! only, never run; its own doc says what that means. `shortcut.rs`
//! and `tray.rs` still refuse, each naming what has to be written and
//! why it is not a file-at-a-time job (a desktop portal or a
//! `StatusNotifierItem` over D-Bus).

pub mod about;
pub mod attention;
pub mod audio;
pub mod autostart;
pub mod color;
pub mod bluetooth;
pub mod capture;
pub mod clipboard;
pub mod devices;
pub mod dialog;
pub mod instance;
pub mod net;
pub mod notify;
pub mod print;
pub mod screen;
pub mod secret;
pub mod shell;
pub mod shortcut;
pub mod system;
pub mod terminal;
pub mod tray;
pub mod window;
