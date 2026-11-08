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
pub mod inside;
pub mod json;
pub mod libraries;

pub mod picked;
pub mod rpc;
pub mod shadow;
pub mod suggest;

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
    /// Whether there is a server to be had at all.
    ///
    /// Asked at the handshake, when the first page has not been opened
    /// yet and there is nothing to analyse: what the editor needs then
    /// is only whether to promise completion.
    fn exists(&self) -> bool {
        match self {
            Self::Held(_) => true,
            Self::None => false,
            Self::Start(command) => std::process::Command::new(command)
                .arg("--version")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|status| status.success()),
        }
    }

    /// Start it, if it has not been started, for the application that
    /// owns `page`.
    ///
    /// One application, not every application of the workspace. Six
    /// crates of the same name, loading at once, left `rust-analyzer`
    /// answering `null` to everything; the single one it was proved
    /// against answers in seconds. And it is started at the first page
    /// opened rather than at the handshake, because that is the first
    /// moment anything says which application is being worked on.
    fn ready_for(&mut self, page: &std::path::Path) {
        if let Self::Start(command) = self {
            let application = crate_for_pages::application_of(page);
            let linked: Vec<std::path::PathBuf> = application
                .as_deref()
                .and_then(crate_for_pages::written_for)
                .into_iter()
                .collect();
            if linked.is_empty() {
                // Nothing to read yet — the application has never been
                // built. Asked again at the next page opened.
                return;
            }
            let root = application
                .as_deref()
                .map(shadow::uri_of)
                .unwrap_or_else(|| "file:///".to_string());
            let root = Some(root.as_str());
            *self = match downstream::Server::start_in(command, root, &linked) {
                Some(server) => {
                    eprintln!(
                        "rustx: completion, hovers and definitions come from `{command}`, \r
                         reading {}.",
                        linked
                            .first()
                            .map(|manifest| manifest.to_string_lossy().to_string())
                            .unwrap_or_else(|| "nothing".to_string())
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

    /// The server downstream, restarting it first if it died.
    ///
    /// `rust-analyzer` can die without the window closing — out of
    /// memory, a panic on reloading a workspace, a kill to free a
    /// machine — and answering nothing forever after that is what a
    /// developer sees as the LSP "faltering": completion, hovers and
    /// definitions silently stop, with no sign of why. A dead child is
    /// restarted instead, and every file already open is told again —
    /// the fresh process otherwise knows nothing until the next
    /// keystroke sends a `didChange` it has no file to apply it to.
    fn held(
        &mut self,
        shadows: &std::collections::HashMap<String, shadow::Shadow>,
    ) -> Option<&mut downstream::Server> {
        if let Self::Held(server) = self
            && !server.alive()
        {
            if server.restart() {
                for shadow in shadows.values() {
                    server.tell(
                        "textDocument/didOpen",
                        &shadow::told(shadow, "textDocument/didOpen"),
                    );
                }
            } else {
                *self = Self::None;
            }
        }
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
    // And the text itself, as the editor last sent it. The markup half
    // of completion reads this rather than the generated file: what is
    // being completed is usually not valid Rust yet, and that is the
    // normal state of a file somebody is typing in.
    let mut sources: std::collections::HashMap<String, String> = std::collections::HashMap::new();

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
                let _ = &root;
                // What is promised is what there is: an editor told a
                // server provides completion asks it, and waits. The
                // server itself starts at the first page opened, which
                // is the first moment anything says which application
                // is being worked on.
                // Completion is promised whatever else is installed: the
                // tags, the attributes, the components and the style
                // keys are the server's own, and they need no
                // `rust-analyzer`, no built project and no compile at
                // all. An editor that was not told so would never ask.
                //
                // The trigger characters are where a developer is when
                // they want a list: `<` opens a tag, a space starts an
                // attribute, `/` closes a tag, and `{`, `.` and `:` are
                // inside an expression.
                let completion = ",\"completionProvider\":{\"triggerCharacters\":\
                                  [\"<\",\" \",\"/\",\"{\",\".\",\":\"]}";
                let assistance = if downstream.exists() {
                    format!("{completion},\"hoverProvider\":true,\"definitionProvider\":true")
                } else {
                    completion.to_string()
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
                // The markup first: it needs nothing but the text, so
                // it answers while `rust-analyzer` is still loading, and
                // it answers for positions that have no counterpart in
                // the generated file at all.
                let answer = match offer(&method, &message, &sources) {
                    Some(items) => items,
                    None => assist(
                        &method,
                        &message,
                        &shadows,
                        &sources,
                        downstream.held(&shadows),
                    ),
                };
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
                sources.insert(uri.clone(), text.clone());

                // The underlining goes out first: it is what a keystroke
                // is waiting for, and the shadow is for the question
                // after it.
                if let Some(fresh) = shadow::of(&uri, &text) {
                    // The first page opened is what says which
                    // application is being worked on, so it is where the
                    // server downstream starts.
                    if let Some(page) = shadow::path_of(&uri) {
                        downstream.ready_for(&page);
                    }
                    let telling = if shadows.contains_key(&uri) {
                        "textDocument/didChange"
                    } else {
                        "textDocument/didOpen"
                    };
                    if let Some(server) = downstream.held(&shadows) {
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

/// What the server offers out of its own head, or nothing when the
/// position is not in the markup.
///
/// `None` means "not mine": the caller then asks `rust-analyzer`
/// through the position map, which is the right answer for a position in
/// the developer's own Rust.
fn offer(
    method: &str,
    message: &str,
    sources: &std::collections::HashMap<String, String>,
) -> Option<String> {
    // A hover or a definition on a tag name has nothing to say that the
    // tag itself does not: the markup half answers completion only.
    if method != "textDocument/completion" {
        return None;
    }
    let uri = json::field(message, "uri")?;
    let source = sources.get(&uri)?;
    let line = json::number(message, "line")?.max(0) as usize + 1;
    let character = json::number(message, "character")?.max(0) as usize + 1;

    let page = match shadow::path_of(&uri) {
        Some(path) => suggest::page_of(source, &path),
        None => suggest::Page {
            components: rustx_compiler::promote::component_signatures(source),
            libraries: Vec::new(),
            application: None,
        },
    };

    let items = match inside::at(source, line, character) {
        inside::Place::Rust => return None,
        inside::Place::TagName { typed } => suggest::tags(&typed, &page),
        inside::Place::ClosingTag { open } => suggest::closing(&open),
        inside::Place::AttributeName {
            tag,
            typed,
            already,
        } => suggest::attributes(&tag, &typed, &page, &already),
        inside::Place::StyleKey { typed } => suggest::style_keys(&typed),
        // In an attribute's value, or in the text between two tags:
        // nothing the server knows, and a list of tags in the middle of
        // a sentence would be worse than nothing.
        inside::Place::AttributeValue { .. } => Vec::new(),
        // The expression's own completions come from `rust-analyzer`.
        inside::Place::Expression { .. } => return None,
    };
    Some(suggest::list(&items))
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
    sources: &std::collections::HashMap<String, String>,
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
    let (asked_line, asked_character) = (line.max(0) as usize + 1, character.max(0) as usize + 1);

    // A position inside a `{…}` has no column in the generated file,
    // because a whole region is emitted on one line. So the file is
    // rebuilt for this one question, with the expression at its own
    // line and column — and then the position needs no translation at
    // all.
    let picked = sources
        .get(&uri)
        .and_then(|source| picked::file(source, &shadow.rust, asked_line, asked_character));
    if let Some(made) = picked {
        let answered = ask_about(server, method, shadow, &made, asked_line, asked_character);
        return answered
            .unwrap_or_else(|| empty.to_string())
            .replace(&shadow.uri, &uri);
    }

    let (line, character) = match shadow
        .map
        .to_generated_position(asked_line, asked_character)
    {
        Some(position) => position,
        // On a line the rewrite rebuilt. The map calls it opaque, and a
        // question with no position used to be answered with nothing --
        // which is every line that reads a value, every handler, every
        // `async` block: precisely the lines where somebody reaches for
        // a crate's name, and what made it look as though the whole
        // path had to be typed before anything was offered. Most of
        // such a line is still the developer's own text, so the column
        // is found by matching what is in front of the cursor.
        None => {
            let page = sources.get(&uri).map(String::as_str).unwrap_or("");
            let page_line = page.lines().nth(asked_line - 1).unwrap_or("");
            let generated_line = shadow.rust.lines().nth(asked_line - 1).unwrap_or("");
            let Some(column) = picked::aligned(page_line, generated_line, asked_character) else {
                // Nothing of the line survived the rewrite: a position
                // next door is not an answer, it is a wrong one.
                return empty.to_string();
            };
            let asked = format!(
                "{{\"textDocument\":{{\"uri\":\"{}\"}},\"position\":{{\"line\":{},\"character\":{}}}}}",
                json::escape(&shadow.uri),
                asked_line - 1,
                column - 1
            );
            let Some(answer) = server.ask(method, &asked) else {
                return empty.to_string();
            };
            let result = cut_result(&answer).unwrap_or(empty);
            // Back by the distance it went out, so a `textEdit` lands
            // on what was typed and not on what the rewrite put there.
            let by = asked_character as isize - column as isize;
            return picked::shifted(&result.replace(&shadow.uri, &uri), asked_line - 1, by);
        }
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

/// Ask `server` about `made` — a file built for this question — and put
/// the real one back afterwards, whatever happened.
///
/// The downstream server holds one text per file; leaving the made one
/// in place would mean the next underlining, the next hover and the next
/// change were all computed against a file nobody wrote.
fn ask_about(
    server: &mut downstream::Server,
    method: &str,
    shadow: &shadow::Shadow,
    made: &str,
    line: usize,
    character: usize,
) -> Option<String> {
    server.tell(
        "textDocument/didChange",
        &shadow::changed(&shadow.uri, made),
    );
    let asked = format!(
        "{{\"textDocument\":{{\"uri\":\"{}\"}},\"position\":{{\"line\":{},\"character\":{}}}}}",
        json::escape(&shadow.uri),
        line - 1,
        character - 1
    );
    // Once, quickly; and if that said nothing, once more with patience.
    // `rust-analyzer` answers `null` to a question about a file it has
    // only just been handed, and the file built for this question is
    // always one of those. Without the second attempt the first list a
    // developer asks for in a session is empty, every session.
    let mut answer = server.ask(method, &asked);
    if answer.as_deref().and_then(cut_result).unwrap_or("null") == "null" {
        answer = server.ask_again(method, &asked);
    }
    // Back to the real file before anything else is asked.
    server.tell(
        "textDocument/didChange",
        &shadow::changed(&shadow.uri, &shadow.rust),
    );
    cut_result(answer.as_deref()?).map(str::to_string)
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
