//! The tree a `rustx` component returns.
//!
//! It is deliberately the smallest thing that can describe a user
//! interface: a tag, a piece of text, a list of siblings, or nothing. The
//! compiler emits calls that build it and the engine walks it to touch the
//! DOM, so it sits in its own crate — the compiler must not depend on CEF,
//! and the engine must not depend on a parser.

mod element;
pub mod diff;
pub mod value;
pub mod route;
pub mod state;
pub mod store;


pub use value::Value;
pub use element::{
    Key,
    AttributeValue, Element, Event, Handler, IntoElement, TagElement, into_element,
};
