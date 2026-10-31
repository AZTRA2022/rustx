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

pub mod crate_for_pages;
pub mod diagnostics;
pub mod downstream;
pub mod json;
pub mod rpc;
pub mod shadow;

/// Answer an editor until it says goodbye.
///
/// Reading and writing are given in, so that a session is a test: a
/// stream of messages in, a stream of answers out, and no editor
/// needed to know whether the right line was underlined.
pub fn serve(input: &mut impl BufRead, output: &mut impl Write) {
    // Whoever built the server built `rust-analyzer` too, or did not.
    // Both are ordinary, and only one of them has completion in it.
    //
    // It is not started here: `rust-analyzer` analyses a workspace, and
    // which workspace is open is something only the editor knows, in the
    // `initialize` it is about to send. Started without one it analyses
    // nothing and answers every question with silence, which looks
    // exactly like a server that was never installed.
    let named =
        std::env::var("RUSTX_RUST_ANALYZER").unwrap_or_else(|_| "rust-analyzer".to_string());
    answer(input, output, Downstream::Start(named));
}

/// The same, with the downstream server handed in.
///
/// That is what makes a session a test: `rust-analyzer` is a process
/// somewhere on a developer's machine, and what this has to get right —
/// which position is asked about, which answer comes back, which file
/// the answer names — is the translation, not the process.
pub fn serve_with(
    input: &mut impl BufRead,
    output: &mut impl Write,
    downstream: Option<downstream::Server>,
) {
    answer(
        input,
        output,
        match downstream {
            Some(server) => Downstream::Held(server),
            None => Downstream::None,
        },
    );
}

/// The server downstream, or the way to get one.
enum Downstream {
    Held(downstream::Server),
    /// A command to start, once the editor has said which project it has
    /// open.
    Start(String),
    None,
}

impl Downstream {
    /// Start it, if it has not been started, in the project the editor
    /// named.
    fn ready(&mut self, root: Option<&str>) {
        if let Self::Start(command) = self {
            // Every application of this workspace gets the little crate
            // that makes its pages readable, before the server starts:
            // `rust-analyzer` is told which projects to load once, and a
            // project named afterwards means a workspace reloaded under
            // the developer's hands.
            let linked = root
                .and_then(shadow::path_of)
                .map(|root| crate_for_pages::for_workspace(&root))
                .unwrap_or_default();
            *self = match downstream::Server::start_in(command, root, &linked) {
                Some(server) => {
                    eprintln!(
                        "rustx: completion, hovers and definitions come from `{command}`, \
                         reading {} application(s) of {}.",
                        linked.len(),
                        root.unwrap_or("no project")
                    );
                    Self::Held(server)
                }
                // Said out loud, on the channel the editor shows: a
                // developer whose completion is missing has no other way
                // to find out that the server it comes from was never
                // there. Which is exactly how the first install of this
                // extension went.
                None => {
                    eprintln!(
                        "rustx: `{command}` did not start or did not answer, so there is no \
                         completion, no hover and no go-to-definition. Install it \
                         (`rustup component add rust-analyzer`) or say where it is with \
                         `rustx.rustAnalyzer.path`. Mistakes are still underlined."
                    );
                    Self::None
                }
            };
        }
    }

    fn held(&mut self) -> Option<&mut downstream::Server> {
        match self {
            Self::Held(server) => Some(server),
            _ => None,
        }
    }
}

fn answer(input: &mut impl BufRead, output: &mut impl Write, downstream: Downstream) {
    let mut downstream = downstream;
    // The page each open document compiles to, by the document's URI.
    let mut shadows: std::collections::HashMap<String, shadow::Shadow> =
        std::collections::HashMap::new();

    while let Some(message) = rpc::read(input) {
        let Some(method) = json::field(&message, "method") else {
            continue;
        };
        match method.as_str() {
            "initialize" => {
                let id = json::number(&message, "id").unwrap_or(0);
                // The project the editor has open, which is the one
                // `rust-analyzer` has to be pointed at.
                let root =
                    json::field(&message, "rootUri").or_else(|| json::field(&message, "uri"));
                downstream.ready(root.as_deref());
                // What is promised is what there is: an editor told a
                // server provides completion asks it, and waits.
                let assistance = if downstream.held().is_some() {
                    ",\"completionProvider\":{\"triggerCharacters\":[\".\",\":\"]},\
                     \"hoverProvider\":true,\"definitionProvider\":true"
                } else {
                    ""
                };
                rpc::write(
                    output,
                    &format!(
                        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{{\"capabilities\":{{\
                         \"textDocumentSync\":1{assistance}}},\"serverInfo\":{{\"name\":\"rustx-lsp\",\
                         \"version\":\"{}\"}}}}}}",
                        env!("CARGO_PKG_VERSION")
                    ),
                );
            }
            // The three a developer asks for with the keyboard. Each one
            // is the same three steps: the position becomes a position in
            // the shadow, the question goes downstream, and what comes
            // back is told where it really is.
            "textDocument/completion" | "textDocument/hover" | "textDocument/definition" => {
                let id = json::number(&message, "id").unwrap_or(0);
                let answer = assist(&method, &message, &shadows, downstream.held());
                rpc::write(
                    output,
                    &format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{answer}}}"),
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

                // The underlining goes out first: it is what a keystroke
                // is waiting for, and the shadow is for the question
                // after it.
                if let Some(fresh) = shadow::of(&uri, &text) {
                    let telling = if shadows.contains_key(&uri) {
                        "textDocument/didChange"
                    } else {
                        "textDocument/didOpen"
                    };
                    if let Some(server) = downstream.held() {
                        server.tell(telling, &shadow::told(&fresh, telling));
                    }
                    shadows.insert(uri.clone(), fresh);
                }
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

/// Ask the downstream server what the editor asked us, about the shadow
/// instead of the page, and say where the answer really is.
///
/// Every refusal is an empty answer rather than an error: an editor that
/// is told a question failed shows the developer a dialog, and "there is
/// nothing to say about the inside of a tag" is not a failure.
fn assist(
    method: &str,
    message: &str,
    shadows: &std::collections::HashMap<String, shadow::Shadow>,
    downstream: Option<&mut downstream::Server>,
) -> String {
    let empty = if method.ends_with("hover") {
        "null"
    } else {
        "[]"
    };
    let (Some(server), Some(uri)) = (downstream, json::field(message, "uri")) else {
        return empty.to_string();
    };
    let Some(shadow) = shadows.get(&uri) else {
        return empty.to_string();
    };

    // LSP counts lines and characters from zero; the map counts both
    // from one, because that is how a developer and a compiler count.
    let (Some(line), Some(character)) = (
        json::number(message, "line"),
        json::number(message, "character"),
    ) else {
        return empty.to_string();
    };
    let Some((line, character)) = shadow
        .map
        .to_generated_position(line.max(0) as usize + 1, character.max(0) as usize + 1)
    else {
        // Inside a tag, or on a line the rewrite rebuilt. A position
        // next door is not an answer, it is a wrong one.
        return empty.to_string();
    };

    let asked = format!(
        "{{\"textDocument\":{{\"uri\":\"{}\"}},\"position\":{{\"line\":{},\"character\":{}}}}}",
        json::escape(&shadow.uri),
        line - 1,
        character - 1
    );
    let Some(answer) = server.ask(method, &asked) else {
        return empty.to_string();
    };

    // What came back is about the shadow; the editor is holding the
    // page. The two files have the same lines — a region puts back the
    // newlines its expression swallowed — so naming the page is the
    // whole translation.
    let result = cut_result(&answer).unwrap_or(empty);
    result.replace(&shadow.uri, &uri)
}

/// The `result` of an answer, as text, braces and brackets balanced.
///
/// Hand-written, like everything else that reads JSON here: what is
/// needed is one value out of one envelope, and the envelope is this
/// server's own.
fn cut_result(answer: &str) -> Option<&str> {
    let at = answer.find("\"result\"")?;
    let rest = answer[at + "\"result\"".len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let opening = rest.chars().next()?;
    let closing = match opening {
        '{' => '}',
        '[' => ']',
        // `null`, a number, a string: it ends where the envelope does.
        _ => {
            let end = rest.rfind('}')?;
            return Some(rest[..end].trim_end().trim_end_matches(','));
        }
    };

    let mut depth = 0_usize;
    let mut inside_text = false;
    let mut escaped = false;
    for (at, letter) in rest.char_indices() {
        if inside_text {
            match letter {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => inside_text = false,
                _ => {}
            }
            continue;
        }
        match letter {
            '"' => inside_text = true,
            letter if letter == opening => depth += 1,
            letter if letter == closing => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[..=at]);
                }
            }
            _ => {}
        }
    }
    None
}
