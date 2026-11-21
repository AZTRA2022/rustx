//! The host of a rustx window drawn by the system's webview.
//!
//! The application's Rust code runs in this process, on the thread that
//! owns the window. The page is a small interpreter: what the code writes
//! to the DOM is a list of commands, sent once per frame; what comes back
//! is an event, addressed to the node the host built. See [`dom`] and
#![allow(clippy::type_complexity)]
//! [`protocol`].

mod app;
pub mod assets;
pub mod base;
pub mod dom;
pub mod dropped;
mod error;
pub mod files;
pub mod foreign;
pub mod frame;
pub mod host;
pub mod mount;
mod panics;
pub mod policy;
pub mod protocol;
pub mod render;
mod renderer;
pub mod router;
pub mod runtime;
pub mod task;
pub mod tree;
mod value;
pub mod watchdog;
pub mod window;

pub use app::{App, AppConfig, Bootstrap, bootstrap};
pub use dom::{Document, DomError, Node};
pub use error::EngineError;
pub use frame::{PostError, Poster, on_frame, poster};
pub use mount::{mount, mount_at_id};
pub use renderer::{
    NativeFuture, inject, natives_script, on_context_created, register_async_native,
    register_native, restrict_natives,
};
pub use value::{NativeError, Value};
