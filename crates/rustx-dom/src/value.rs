//! What crosses between Rust and JavaScript.
//!
//! Deliberately small: null, a boolean, a number, a string. Nothing
//! borrowed, nothing with a lifetime, nothing holding a `Drop` — a value
//! that crosses the boundary leaves Rust's ownership behind, so the only
//! things allowed across are the ones that can be copied in full.
//!
//! It lives here rather than in the engine because an [`crate::Element`]
//! carries it now: the props of a node React owns are values, and
//! `rustx-dom` cannot depend on the engine. `rustx_engine::Value` is a
//! re-export of this type, so nothing that used it changed.

/// A value a native function can take or return, and a prop a React
/// component can be given.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
}
