//! `rustx-engine` — the layer that hosts the web engine and makes the
//! application's Rust code live inside the render process.

mod app;
mod bootstrap;
mod cef_app;
mod init;
mod client;
mod error;

pub use app::{App, AppConfig};
pub use error::EngineError;
pub use bootstrap::{Bootstrap, Role, bootstrap, process};
pub use init::declare_api_version;
