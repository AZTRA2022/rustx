//! What Windows can do, and how.
//!
//! One directory per platform, and one file per capability inside it:
//! the API a page sees lives one level up, in `clipboard.rs` and
//! `devices.rs`, and says nothing about any platform. So a capability
//! arrives on a second platform by writing one file whose functions
//! have the same names — `linux/clipboard.rs` is the same list as
//! this one — and nothing above changes.

pub mod about;
pub mod attention;
pub mod audio;
pub mod autostart;
pub mod bluetooth;
pub mod capture;
pub mod clipboard;
pub mod color;
pub mod devices;
pub mod dialog;
pub mod http;
pub mod instance;
pub mod media;
pub mod menu;
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
