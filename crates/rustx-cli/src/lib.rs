//! `rustx`: the command a developer types.
//!
//! It reinvents nothing. Building is cargo, and compiling the pages is
//! the attribute macro's job already. What is here is what cargo does
//! not know about: a project skeleton that is right the first time, a
//! loop that watches the files and brings the window back, the state
//! that crosses that restart, and the Windows resources stamped into
//! the `.exe` after the link — so that a built application carries its
//! own name and icon without a `build.rs` in the developer's project,
//! which is precisely what this framework refuses.

pub mod arguments;
pub mod new;
