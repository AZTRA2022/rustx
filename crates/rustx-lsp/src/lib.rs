//! What an editor asks about a `.rsx`.
//!
//! A language server, small on purpose: it compiles the file the editor
//! is holding and says where the compiler refused it, in the
//! coordinates of the `.rsx` — which is where `CompileError` already
//! points, because §7.3 asks the compiler to keep them.
//!
//! What it does not do yet is the other half of §7.7: completion,
//! hovers and definitions, which come from putting `rust-analyzer`
//! behind this same server and translating its answers back through the
//! position map. That is its own plan.

use std::io::{BufRead, Write};

pub mod diagnostics;
pub mod downstream;
pub mod json;
pub mod rpc;

/// Answer an editor until it says goodbye.
///
/// Reading and writing are given in, so that a session is a test: a
/// stream of messages in, a stream of answers out, and no editor
/// needed to know whether the right line was underlined.
pub fn serve(input: &mut impl BufRead, output: &mut impl Write) {
    while let Some(message) = rpc::read(input) {
        let Some(method) = json::field(&message, "method") else {
            continue;
        };
        match method.as_str() {
            "initialize" => {
                let id = json::number(&message, "id").unwrap_or(0);
                rpc::write(
                    output,
                    &format!(
                        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{{\"capabilities\":{{\
                         \"textDocumentSync\":1}},\"serverInfo\":{{\"name\":\"rustx-lsp\",\
                         \"version\":\"{}\"}}}}}}",
                        env!("CARGO_PKG_VERSION")
                    ),
                );
            }
            // The whole text arrives on open and on every change, which
            // is what `textDocumentSync: 1` asked for: a compiler reads
            // a file from its first byte, and a list of edits would have
            // to be applied before it could.
            "textDocument/didOpen" | "textDocument/didChange" => {
                let (Some(uri), Some(text)) =
                    (json::field(&message, "uri"), json::field(&message, "text"))
                else {
                    continue;
                };
                let found = diagnostics::of(&text);
                rpc::write(output, &diagnostics::notification(&uri, &found));
            }
            "shutdown" => {
                let id = json::number(&message, "id").unwrap_or(0);
                rpc::write(
                    output,
                    &format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":null}}"),
                );
            }
            // The editor is closing this window. Anything after it is
            // not ours to answer.
            "exit" => return,
            _ => {}
        }
    }
}
