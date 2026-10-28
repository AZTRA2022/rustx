//! The `.rsx` compiler.
//!
//! A `.rsx` file is Rust with JSX written bare inside it — no wrapping
//! macro, because a macro is a wall that `rust-analyzer` cannot see
//! through, and seeing through it is the whole point. This crate finds the
//! JSX, parses it, emits ordinary Rust that builds a `rustx_dom::Element`,
//! and keeps a map so that a `rustc` error lands on the line the developer
//! wrote rather than on a line a build script invented.

mod error;
pub mod ast;
pub mod attributes;
pub mod build;
pub mod emit;
pub mod map;
pub mod parse;
pub mod promote;
pub mod rewrite;
pub mod transpile;
pub mod scan;
pub mod tags;

pub use error::CompileError;
pub use scan::Span;
