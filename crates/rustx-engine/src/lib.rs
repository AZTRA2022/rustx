//! `rustx-engine` — the layer that hosts the web engine and makes the
//! application's Rust code live inside the render process.

// Every safety net in this engine is a `catch_unwind`: the C boundary
// CEF calls across (`panics::contain`), the frame loop, a job a worker
// posted, a write a future posted, a page that falls over and is
// replaced by an error screen. `panic = "abort"` turns each of them
// into an abort of the whole process, silently — so an application
// built that way has none of the tolerance this engine is written to
// provide, and the first panic in anybody's `onClick` takes the window
// down with no message.
//
// Said here rather than discovered there. An application that genuinely
// wants `abort` wants a different engine.
#[cfg(panic = "abort")]
compile_error!(
    "rustx-engine needs unwinding. Its tolerance for a panic in an event \
     handler, a frame callback, a background task or a page is built on \
     `catch_unwind`, and `panic = \"abort\"` turns every one of those into \
     an abort of the process. Remove `panic = \"abort\"` from this \
     profile."
);

mod app;
mod bootstrap;
pub mod bridge;
mod cef_app;
mod client;
pub mod dom;
pub mod dropped;
mod error;
pub mod foreign;
pub mod frame;
mod guard;
mod init;
pub mod mount;
mod panics;
pub mod render;
pub mod renderer;
pub mod router;
pub mod runtime;
pub mod task;
pub mod tree;
mod value;
pub mod watchdog;
pub mod window;

pub use app::{App, AppConfig};
pub use bootstrap::{Bootstrap, Role, bootstrap, process};
pub use dom::{Document, DomError, Node};
pub use error::EngineError;
pub use frame::{PostError, Poster, on_frame, poster};
pub use guard::{is_local, refusals};
pub use init::declare_api_version;
pub use mount::{mount, mount_at_id};
pub use renderer::{inject, on_context_created, register_native};
pub use value::{NativeError, Value};
