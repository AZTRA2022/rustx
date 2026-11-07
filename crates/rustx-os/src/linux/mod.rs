//! What Linux can do, and how.
//!
//! The same files as `windows/`, with the same function names, so that
//! what is above this directory never asks which platform it is on.
//! They refuse for now, each naming what has to be written — see
//! `clipboard.rs` and `devices.rs` beside this file.

pub mod autostart;
pub mod clipboard;
pub mod devices;
pub mod dialog;
pub mod instance;
pub mod notify;
pub mod shortcut;
pub mod tray;
pub mod window;
