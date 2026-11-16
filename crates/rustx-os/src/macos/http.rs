//! HTTP on macOS: the system's own client program.
//!
//! The whole of it is in [`crate::piped`], which is shared with the
//! other Unix: both systems keep their HTTP in a program rather than in
//! a library that can be called without linking it, and both have that
//! program. What this file is, is the statement that it is the answer
//! here -- Windows has WinHTTP and calls it directly, in
//! `windows/http.rs`.
//!
//! What it gets by going through `curl`: the system's certificate
//! store, the system's proxy settings, redirects, and timeouts. What it
//! costs: one process per request, which for an update check is
//! nothing.

#[allow(unused_imports)]
pub use crate::piped::{download, get};
