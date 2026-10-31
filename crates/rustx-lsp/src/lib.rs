//! What an editor asks about a `.rsx`.
//!
//! A language server, small on purpose: it compiles the file the editor
//! is holding and says where the compiler refused it, in the
//! coordinates of the `.rsx` — which is where `CompileError` already
//! points, because §7.3 asks the compiler to keep them.
//!
//! This file holds nothing itself: reading and writing LSP is `rpc`,
//! and the little JSON that needs is `json`.

pub mod json;
pub mod rpc;
