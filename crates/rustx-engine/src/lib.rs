//! `rustx-engine` — the layer that hosts the web engine and makes the
//! application's Rust code live inside the render process.

mod app;
mod bootstrap;
mod cef_app;
mod client;
pub mod dom;
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
pub mod task;
pub mod tree;
mod value;

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
