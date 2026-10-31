//! What Linux can do, and how.
//!
//! The same files as `windows/`, with the same function names, so that
//! what is above this directory never asks which platform it is on.
//! They refuse for now, each naming what has to be written — see
//! `clipboard.rs` and `devices.rs` beside this file.

pub mod clipboard;
pub mod devices;
