//! The edge between Rust and C, where a panic may not pass.
//!
//! CEF calls into the engine through C function pointers. A panic that
//! unwinds across one of those is undefined behaviour — not a crash with a
//! backtrace, but whatever the C++ side happens to do with a stack it never
//! agreed to unwind. Every function CEF can call must therefore catch its
//! own panics and turn them into a message.
//!
//! The engine does not abort on one. A panic in an application callback is
//! a bug in that callback, and a framework whose first application drives
//! hardware has to stay on screen long enough to say so.

use std::panic::{AssertUnwindSafe, catch_unwind};

/// Run `body`, and report rather than unwind if it panics.
pub(crate) fn contain(what: &str, body: impl FnOnce()) {
    if catch_unwind(AssertUnwindSafe(body)).is_err() {
        eprintln!("rustx: {what} panicked; the engine carried on");
    }
}
