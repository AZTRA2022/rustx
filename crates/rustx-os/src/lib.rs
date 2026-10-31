//! What a program needs from the machine.
//!
//! This is the layer that separates `rustx` from a faster Tauri: the
//! devices, the camera, the screen, recording. It knows nothing about
//! the DOM, nothing about CEF and nothing about components — it is
//! hardware and the words for it, so that the engine above can stay
//! about drawing.
//!
//! # Windows first
//!
//! Every capability here is Windows today. The other platforms compile,
//! and refuse in a sentence that names the platform: a program that
//! builds everywhere and silently does nothing on two of the three is
//! worse than one that says where it stands.
//!
//! # One dependency
//!
//! `windows`, Microsoft's own bindings, and only in this crate. The
//! design document asks for an existing crate wherever there is a good
//! one (§7.6), and hand-writing the COM vtables of Media Foundation is
//! precisely the work this project has no reason to redo. Nothing above
//! this crate hears about it.

pub mod devices;
pub mod error;
#[cfg(windows)]
mod windows_media;

pub use error::OsError;
