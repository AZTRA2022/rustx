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
pub mod documents;
pub mod downstream;
pub mod inside;
pub mod json;
pub mod libraries;

pub mod picked;
pub mod rpc;
pub mod shadow;
pub mod suggest;
pub mod wire;

/// Do something that reads a half-typed page, and survive it.
///
/// Every one of these is reached from a keystroke, on text that is
/// mid-edit and therefore not valid anything. The compiler is called on
/// that text — to underline it, to write the shadow, to find the region
/// around the cursor — and a compiler that panics on a half-written tag
/// would take this process with it. A server that died once underlines
/// nothing for the rest of the session, in every file, with no message
/// saying why: the one failure a developer cannot work around and cannot
/// diagnose.
///
/// So nothing a message handler computes is allowed to escape. `None`
/// means "that could not be worked out", which every caller here
/// already had to handle.
pub fn safely<T>(what: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(what)).ok()
}

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
    // And the text itself, as the editor last sent it — the one thing in
    // this process that has to agree with the editor's window byte for
    // byte. The markup half of completion reads this rather than the
    // generated file: what is being completed is usually not valid Rust
    // yet, and that is the normal state of a file somebody is typing in.
    let mut documents = documents::Documents::new();
    // A document whose text has moved on but whose underlines have not
    // gone out yet, because more messages were already in the pipe. It
    // is flushed before this loop blocks on the pipe again, and before
    // any question that reads a shadow — so a burst of keystrokes costs
    // one compile instead of twenty, and nothing is ever left unsaid.
    let mut deferred: Option<String> = None;

    let mut frames = rpc::Frames::over(input);
    loop {
        if !frames.waiting()
            && let Some(uri) = deferred.take()
        {
            published(output, &uri, &documents, &mut shadows, &mut downstream);
        }
        let Some(body) = frames.message() else {
            return;
        };
        // Parsed, rather than searched for a quoted name. A `.rsx` is
        // allowed to contain the characters `"text":` and `"uri":`, and a
        // reader that took the document's own for the message's handed
        // the compiler a fragment of the page as the whole of it.
        let Some(message) = wire::parse(&body) else {
            continue;
        };
        let Some(method) = message.get("method").as_str().map(str::to_string) else {
            continue;
        };
        let params = message.get("params");
        let asked = message.get("id");
        // An id may be a number or, by the specification, a string, and
        // the answer must carry back the one that was sent: an editor
        // matches on it, and waits for ever on a question it cannot
        // match.
        let id = match asked {
            wire::Value::Text(text) => format!("\"{}\"", json::escape(text)),
            other => other.as_i64().unwrap_or(0).to_string(),
        };
        let uri_of_document = params
            .get("textDocument")
            .get("uri")
            .as_str()
            .map(str::to_string);

        // Whatever was deferred goes out unless this very message is the
        // next keystroke of the same burst. One rule, in one place: a
        // burst that ended in a `shutdown`, a question, a close, or
        // anything else at all used to lose its last change — and the
        // last change of a burst is the text on the developer's screen.
        let continues_the_burst = method == "textDocument/didChange"
            && deferred.is_some()
            && deferred.as_deref() == uri_of_document.as_deref();
        if !continues_the_burst && let Some(uri) = deferred.take() {
            published(output, &uri, &documents, &mut shadows, &mut downstream);
        }

        match method.as_str() {
            "initialize" => {
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
                // `resolveProvider` only when there is a server behind
                // this one to resolve with: a developer with no
                // `rust-analyzer` must never be promised an import
                // edit nobody will ever supply.
                let completion_with_resolve = ",\"completionProvider\":{\"triggerCharacters\":\
                                  [\"<\",\" \",\"/\",\"{\",\".\",\":\"],\
                                  \"resolveProvider\":true}";
                // Everything else `rust-analyzer` answers, promised
                // only when there is a `rust-analyzer` to answer it.
                //
                // They are all the same round trip as `definition`,
                // and they are correct for one reason worth writing
                // down: a generated page keeps its own line numbers.
                // `every_page_keeps_its_line_count` is a law of the
                // compiler, so line *N* of the shadow is line *N* of
                // the `.rsx`, and a range that comes back needs no
                // translation at all. Columns are the only thing that
                // moves, and only on the lines the rewrite rebuilt.
                //
                // Not here, and each for a reason:
                // `rename` and `codeAction`, because both hand back
                // edits with columns in them and a column that is
                // wrong by two characters rewrites the wrong part of
                // somebody's file; and `formatting`, because the text
                // being formatted is generated Rust and the answer
                // would be a `.rsx` reformatted as Rust.
                let more = "\"hoverProvider\":true,\"definitionProvider\":true,\
                            \"typeDefinitionProvider\":true,\"implementationProvider\":true,\
                            \"referencesProvider\":true,\"documentHighlightProvider\":true,\
                            \"documentSymbolProvider\":true,\"foldingRangeProvider\":true,\
                            \"inlayHintProvider\":true,\
                            \"signatureHelpProvider\":{\"triggerCharacters\":[\"(\",\",\"]}";
                let assistance = if downstream.exists() {
                    format!("{completion_with_resolve},{more}")
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
            // Whole-file questions: no position in them, so the
            // shadow answers directly. `rust-analyzer` already holds
            // the shadow's current text -- every question that swaps it
            // puts it back -- so nothing has to be handed over first.
            "textDocument/documentSymbol"
            | "textDocument/foldingRange"
            | "textDocument/inlayHint" => {
                let answer =
                    safely(|| about_the_file(&method, &body, &shadows, downstream.held(&shadows)))
                        .flatten()
                        .unwrap_or_else(|| "[]".to_string());
                rpc::write(
                    output,
                    &format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{answer}}}"),
                );
            }
            "textDocument/completion"
            | "textDocument/hover"
            | "textDocument/definition"
            | "textDocument/typeDefinition"
            | "textDocument/implementation"
            | "textDocument/references"
            | "textDocument/documentHighlight"
            | "textDocument/signatureHelp" => {
                // The markup first: it needs nothing but the text, so
                // it answers while `rust-analyzer` is still loading, and
                // it answers for positions that have no counterpart in
                // the generated file at all.
                let answer = match safely(|| offer(&method, params, &documents)).flatten() {
                    Some(items) => items,
                    None => safely(|| {
                        assist(
                            &method,
                            params,
                            &shadows,
                            &documents,
                            downstream.held(&shadows),
                        )
                    })
                    .unwrap_or_else(|| empty_for(&method).to_string()),
                };
                // `completionItem/resolve` carries no document at all —
                // only whatever `data` the item already has — so the
                // page it came from has to travel on the item itself.
                let answer = if method == "textDocument/completion" {
                    match &uri_of_document {
                        Some(uri) => safely(|| json::tag_items(&answer, uri))
                            .unwrap_or_else(|| answer.clone()),
                        None => answer,
                    }
                } else {
                    answer
                };
                rpc::write(
                    output,
                    &format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{answer}}}"),
                );
            }
            // The whole text arrives on open, and on every change too
            // unless a client has decided otherwise: `textDocumentSync:
            // 1` is what this server asks for, because a compiler reads
            // a file from its first byte. A client that sends ranges
            // anyway is applied correctly rather than misread.
            "textDocument/didOpen" => {
                let (Some(uri), Some(text)) = (
                    uri_of_document,
                    params.get("textDocument").get("text").as_str(),
                ) else {
                    continue;
                };
                let version = params
                    .get("textDocument")
                    .get("version")
                    .as_i64()
                    .unwrap_or(0);
                documents.opened(&uri, text.to_string(), version);
                published(output, &uri, &documents, &mut shadows, &mut downstream);
            }
            "textDocument/didChange" => {
                let Some(uri) = uri_of_document else {
                    continue;
                };
                let version = params.get("textDocument").get("version").as_i64();
                let changes = documents::changes_in(params);
                if documents.changed(&uri, version, &changes) == documents::Applied::Ignored {
                    continue;
                }
                // Somebody typing quickly puts several notifications in
                // the pipe before the first has been answered, and every
                // one but the last describes a text that is no longer on
                // screen. The change itself is always applied; only the
                // compiling waits for the end of the burst.
                if frames.waiting() {
                    deferred = Some(uri);
                } else {
                    published(output, &uri, &documents, &mut shadows, &mut downstream);
                }
            }
            // The editor has closed the file. Everything held for it goes
            // — and its underlines are withdrawn, because an editor keeps
            // showing what a server published until the server says
            // there is nothing, and a closed file that still has a red
            // squiggle in the problems panel is a lie that survives
            // until the window is reloaded.
            "textDocument/didClose" => {
                let Some(uri) = uri_of_document else {
                    continue;
                };
                let was_open = documents.closed(&uri);
                let shadow = shadows.remove(&uri);
                if was_open {
                    rpc::write(output, &diagnostics::notification(&uri, &[]));
                }
                // And the downstream server is told too, or it keeps a
                // text nobody has open and answers about it.
                if let Some(shadow) = shadow
                    && let Some(server) = downstream.held(&shadows)
                {
                    server.tell(
                        "textDocument/didClose",
                        &format!(
                            "{{\"textDocument\":{{\"uri\":\"{}\"}}}}",
                            json::escape(&shadow.uri)
                        ),
                    );
                }
            }
            // The item is `params` itself, with no envelope and no
            // document attached to the question — the tag this server
            // wrote onto it (Task 2) is the only way back to a shadow.
            "completionItem/resolve" => {
                // Forwarded as it was written, which is why this one is
                // read out of the raw body: an item carries `data` that
                // belongs to `rust-analyzer`, and it has to go back
                // exactly as it came.
                let item = json::object_field(&body, "params").unwrap_or("{}");
                let answer = safely(|| {
                    let (untagged, uri) = json::untag_item(item);
                    resolved(
                        &untagged,
                        uri.as_deref(),
                        &shadows,
                        downstream.held(&shadows),
                    )
                })
                .unwrap_or_else(|| item.to_string());
                rpc::write(
                    output,
                    &format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{answer}}}"),
                );
            }
            "shutdown" => {
                rpc::write(
                    output,
                    &format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":null}}"),
                );
            }
            // The editor is closing this window. Anything after it is
            // not ours to answer.
            "exit" => return,
            // Something this server does not do. A notification is
            // ignored, which is what the specification asks for; a
            // *request* is refused, because a request with no answer is
            // an editor waiting on a spinner until it times out.
            _ => {
                if !asked.is_null() {
                    rpc::write(
                        output,
                        &format!(
                            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":-32601,\
                             \"message\":\"rustx-lsp does not answer {}\"}}}}",
                            json::escape(&method)
                        ),
                    );
                }
            }
        }
    }
}

/// Underline a document, and write the Rust that stands in for it.
///
/// The underlining goes out first: it is what a keystroke is waiting
/// for, and the shadow is for the question after it. Both are wrapped,
/// because both call the compiler on a text that is mid-edit and
/// therefore not valid anything.
fn published(
    output: &mut impl Write,
    uri: &str,
    documents: &documents::Documents,
    shadows: &mut std::collections::HashMap<String, shadow::Shadow>,
    downstream: &mut Downstream,
) {
    let Some(text) = documents.text(uri) else {
        return;
    };
    let found = safely(|| diagnostics::of(text)).unwrap_or_default();
    rpc::write(output, &diagnostics::notification(uri, &found));

    if let Some(fresh) = safely(|| shadow::of(uri, text)).flatten() {
        // The first page opened is what says which application is being
        // worked on, so it is where the server downstream starts.
        if let Some(page) = shadow::path_of(uri) {
            downstream.ready_for(&page);
        }
        let telling = if shadows.contains_key(uri) {
            "textDocument/didChange"
        } else {
            "textDocument/didOpen"
        };
        if let Some(server) = downstream.held(shadows) {
            server.tell(telling, &shadow::told(&fresh, telling));
        }
        shadows.insert(uri.to_string(), fresh);
    }
}

/// What an answer is when there is nothing to say: `null` for a hover,
/// an empty list for anything else.
///
/// Never an error. An editor told a question failed shows the developer
/// a dialog, and "there is nothing to say about the inside of a tag" is
/// not a failure.
fn empty_for(method: &str) -> &'static str {
    // A client reads `null` and `[]` differently, and handing back the
    // wrong empty is how a popup gets stuck on screen: VS Code keeps
    // the last signature help it was shown until it is told `null`,
    // and reads `[]` as "an answer with no items in it".
    if method.ends_with("hover") || method.ends_with("signatureHelp") {
        "null"
    } else {
        "[]"
    }
}

/// Ask `rust-analyzer` a question about the whole file.
///
/// `documentSymbol`, `foldingRange` and `inlayHint` carry no position
/// -- the first two carry nothing but a document and the third a range
/// -- so the question goes over with the shadow's URI in place of the
/// page's, and what comes back needs only the URI put back. The lines
/// are already the page's own: see the note in `initialize`.
fn about_the_file(
    method: &str,
    body: &str,
    shadows: &std::collections::HashMap<String, shadow::Shadow>,
    downstream: Option<&mut downstream::Server>,
) -> Option<String> {
    let params = json::object_field(body, "params")?;
    let server = downstream?;
    // The URI as the client wrote it, taken from the text rather than
    // parsed, so that what goes downstream is the client's own
    // parameters with one string changed.
    let uri = {
        let value = wire::parse(params)?;
        value.get("textDocument").get("uri").as_str()?.to_string()
    };
    let shadow = shadows.get(&uri)?;
    let asked = params.replace(&json::escape(&uri), &json::escape(&shadow.uri));
    let answer = server.ask(method, &asked)?;
    let result = cut_result(&answer)?;
    if result == "null" {
        return None;
    }
    Some(result.replace(&json::escape(&shadow.uri), &json::escape(&uri)))
}

/// What the server offers out of its own head, or nothing when the
/// position is not in the markup.
///
/// `None` means "not mine": the caller then asks `rust-analyzer`
/// through the position map, which is the right answer for a position in
/// the developer's own Rust.
fn offer(method: &str, params: &wire::Value, documents: &documents::Documents) -> Option<String> {
    // A hover or a definition on a tag name has nothing to say that the
    // tag itself does not: the markup half answers completion only.
    if method != "textDocument/completion" {
        return None;
    }
    let uri = params.get("textDocument").get("uri").as_str()?;
    let source = documents.text(uri)?;
    let asked = documents::position_in(params.get("position"))?;
    // LSP counts from zero and in UTF-16 units; everything inside this
    // server counts from one and in characters, because that is how a
    // developer and a compiler count. An emoji earlier on the line is
    // two units and one character, and a position that was not
    // converted landed in the wrong word.
    let line = asked.line + 1;
    let character = documents::character_in(source, asked.line, asked.character) + 1;

    let page = match shadow::path_of(uri) {
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
    params: &wire::Value,
    shadows: &std::collections::HashMap<String, shadow::Shadow>,
    documents: &documents::Documents,
    downstream: Option<&mut downstream::Server>,
) -> String {
    let empty = empty_for(method);
    let (Some(server), Some(uri)) = (downstream, params.get("textDocument").get("uri").as_str())
    else {
        return empty.to_string();
    };
    let Some(shadow) = shadows.get(uri) else {
        return empty.to_string();
    };
    let Some(asked) = documents::position_in(params.get("position")) else {
        return empty.to_string();
    };

    // LSP counts lines from zero and characters in UTF-16 units; the map
    // counts lines from one and characters in characters, because that
    // is how a developer and a compiler count.
    let source = documents.text(uri).unwrap_or("");
    let asked_line = asked.line + 1;
    let asked_character = documents::character_in(source, asked.line, asked.character) + 1;

    // A position inside a `{…}` has no column in the generated file,
    // because a whole region is emitted on one line. So the file is
    // rebuilt for this one question, with the expression at its own
    // line and column — and then the position needs no translation at
    // all.
    let picked = picked::file(source, &shadow.rust, asked_line, asked_character);
    if let Some(made) = picked {
        let answered = ask_about(server, method, shadow, &made, asked_line, asked_character);
        return answered
            .unwrap_or_else(|| empty.to_string())
            .replace(&shadow.uri, uri);
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
            let page_line = source.lines().nth(asked_line - 1).unwrap_or("");
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
            return picked::shifted(&result.replace(&shadow.uri, uri), asked_line - 1, by);
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
    result.replace(&shadow.uri, uri)
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

/// Answer `completionItem/resolve` about `item`, already untagged.
///
/// Unchanged whenever there is nowhere to ask: no tag at all (the
/// markup's own items, or an item a client resolves twice), no shadow
/// for the tagged URI, or no live server to ask.
fn resolved(
    item: &str,
    uri: Option<&str>,
    shadows: &std::collections::HashMap<String, shadow::Shadow>,
    downstream: Option<&mut downstream::Server>,
) -> String {
    let (Some(uri), Some(server)) = (uri, downstream) else {
        return item.to_string();
    };
    let Some(shadow) = shadows.get(uri) else {
        return item.to_string();
    };
    let Some(answer) = server.ask("completionItem/resolve", item) else {
        return item.to_string();
    };
    let result = cut_result(&answer).unwrap_or(item);
    retarget_edits(result, &shadow.map)
}

/// Every `additionalTextEdits` entry of a resolved item, each range
/// translated from the shadow's coordinates to the `.rsx`'s. An edit
/// whose start or end has no equivalent there is dropped outright: a
/// wrong position is worse than a missing one (Review Focus 1 of the
/// plan this answers).
fn retarget_edits(item: &str, map: &rustx_compiler::map::PositionMap) -> String {
    let Some(edits) = json::array_field(item, "additionalTextEdits") else {
        return item.to_string();
    };
    let inside = &edits[1..edits.len().saturating_sub(1)];
    let kept: Vec<String> = json::objects_in(inside)
        .into_iter()
        .filter_map(|(start, end)| retargeted_edit(&inside[start..end], map))
        .collect();
    let rebuilt = format!("[{}]", kept.join(","));
    json::replace_field(item, "additionalTextEdits", '[', ']', &rebuilt)
}

/// One `additionalTextEdits` entry, its range translated — `None` when
/// either end has no `.rsx` equivalent at all.
fn retargeted_edit(edit: &str, map: &rustx_compiler::map::PositionMap) -> Option<String> {
    let range = json::object_field(edit, "range")?;
    let start = json::object_field(range, "start")?;
    let end = json::object_field(range, "end")?;
    let (start_line, start_character) = (
        json::number(start, "line")?,
        json::number(start, "character")?,
    );
    let (end_line, end_character) = (json::number(end, "line")?, json::number(end, "character")?);
    // LSP counts lines and characters from zero; the map, like a
    // developer, counts both from one.
    let (rsx_start_line, rsx_start_character) =
        map.to_rsx_position(start_line as usize + 1, start_character as usize + 1)?;
    let (rsx_end_line, rsx_end_character) =
        map.to_rsx_position(end_line as usize + 1, end_character as usize + 1)?;
    let new_text = json::field(edit, "newText").unwrap_or_default();
    Some(format!(
        "{{\"range\":{{\"start\":{{\"line\":{},\"character\":{}}},\"end\":{{\"line\":{},\
         \"character\":{}}}}},\"newText\":\"{}\"}}",
        rsx_start_line - 1,
        rsx_start_character - 1,
        rsx_end_line - 1,
        rsx_end_character - 1,
        json::escape(&new_text),
    ))
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
                // Below zero would be a panic in a debug build and a
                // wrapped count in a release one; neither is an answer.
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&rest[..=at]);
                }
            }
            _ => {}
        }
    }
    None
}
