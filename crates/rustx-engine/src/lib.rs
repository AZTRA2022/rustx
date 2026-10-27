//! `rustx-engine` — the layer that hosts the web engine and makes the
//! application's Rust code live inside the render process.

mod app;
mod bootstrap;
mod cef_app;
mod client;
pub mod dom;
mod error;
mod init;
pub mod renderer;
mod value;

pub use app::{App, AppConfig};
pub use bootstrap::{Bootstrap, Role, bootstrap, process};
pub use error::EngineError;
pub use init::declare_api_version;
pub use renderer::{on_context_created, register_native};
pub use dom::{Document, DomError, Node};
pub use value::{NativeError, Value};
