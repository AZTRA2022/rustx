//! The server driven over its own protocol.
//!
//! Not by calling its functions: by framing real JSON-RPC, writing it
//! into the server's standard input and reading what comes out of its
//! standard output. That is the only way to test the things that
//! actually break — a message read wrongly, a position published one
//! column out, an underline left behind after the file was closed —
//! because every one of them lives in the translation between the wire
//! and the compiler, which is exactly what calling a function skips.
//!
//! A session is therefore: a stream of messages in, a stream of answers
//! out, and a set of assertions about the answers. No editor required.

use rustx_lsp::{json, serve, wire};

/// A conversation being built.
struct Session {
    stream: Vec<u8>,
    version: i64,
}

/// Everything the server said.
struct Said(Vec<String>);

impl Session {
    fn new() -> Self {
        Self {
            stream: Vec::new(),
            version: 1,
        }
    }

    /// A session that has shaken hands, which is how every real one
    /// starts.
    fn started() -> Self {
        let mut session = Self::new();
        session.send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
        session.send(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#);
        session
    }

    fn send(&mut self, body: &str) -> &mut Self {
        self.stream.extend_from_slice(
            format!("Content-Length: {}\r\n\r\n{body}", body.len()).as_bytes(),
        );
        self
    }

    /// Bytes that are not a message at all, to prove a session survives
    /// one.
    fn send_raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.stream.extend_from_slice(bytes);
        self
    }

    fn open(&mut self, uri: &str, text: &str) -> &mut Self {
        self.version = 1;
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"rsx","version":1,"text":"{}"}}}}}}"#,
            json::escape(uri),
            json::escape(text)
        );
        self.send(&body)
    }

    /// A change carrying the whole text, which is what this server asks
    /// its clients for.
    fn replace(&mut self, uri: &str, text: &str) -> &mut Self {
        self.version += 1;
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{}","version":{}}},"contentChanges":[{{"text":"{}"}}]}}}}"#,
            json::escape(uri),
            self.version,
            json::escape(text)
        );
        self.send(&body)
    }

    /// A change carrying a range, which is what a client that decided to
    /// send ranges sends.
    fn edit(
        &mut self,
        uri: &str,
        start: (usize, usize),
        end: (usize, usize),
        text: &str,
    ) -> &mut Self {
        self.version += 1;
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{}","version":{}}},"contentChanges":[{{"range":{{"start":{{"line":{},"character":{}}},"end":{{"line":{},"character":{}}}}},"rangeLength":0,"text":"{}"}}]}}}}"#,
            json::escape(uri),
            self.version,
            start.0,
            start.1,
            end.0,
            end.1,
            json::escape(text)
        );
        self.send(&body)
    }

    fn close(&mut self, uri: &str) -> &mut Self {
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didClose","params":{{"textDocument":{{"uri":"{}"}}}}}}"#,
            json::escape(uri)
        );
        self.send(&body)
    }

    fn request(&mut self, id: i64, method: &str, params: &str) -> &mut Self {
        let body = format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"{}","params":{params}}}"#,
            json::escape(method)
        );
        self.send(&body)
    }

    /// `shutdown` and `exit`, which is how every real session ends — and
    /// the proof that the server was still answering at the end of it.
    fn ended(&mut self) -> Said {
        self.request(9999, "shutdown", "null");
        self.send(r#"{"jsonrpc":"2.0","method":"exit"}"#);
        self.run()
    }

    fn run(&self) -> Said {
        let mut output: Vec<u8> = Vec::new();
        serve(
            &mut std::io::Cursor::new(self.stream.clone()),
            &mut output,
        );
        Said(bodies(&output))
    }
}

/// What the server wrote, split back into message bodies.
fn bodies(output: &[u8]) -> Vec<String> {
    let said = String::from_utf8_lossy(output).to_string();
    let mut found = Vec::new();
    let mut rest = said.as_str();
    while let Some(at) = rest.find("Content-Length: ") {
        let after = &rest[at + "Content-Length: ".len()..];
        let Some((length, remainder)) = after.split_once("\r\n\r\n") else {
            break;
        };
        let Ok(length) = length.trim().parse::<usize>() else {
            break;
        };
        if remainder.len() < length {
            break;
        }
        found.push(remainder[..length].to_string());
        rest = &remainder[length..];
    }
    found
}

/// One published diagnostic, as the editor would read it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Underline {
    line: usize,
    from: usize,
    to: usize,
    message: String,
}

impl Said {
    /// Every `publishDiagnostics` for this URI, in the order they were
    /// sent.
    fn publications(&self, uri: &str) -> Vec<Vec<Underline>> {
        let mut found = Vec::new();
        for body in &self.0 {
            let Some(message) = wire::parse(body) else {
                continue;
            };
            if message.get("method").as_str() != Some("textDocument/publishDiagnostics") {
                continue;
            }
            if message.get("params").get("uri").as_str() != Some(uri) {
                continue;
            }
            found.push(
                message
                    .get("params")
                    .get("diagnostics")
                    .items()
                    .iter()
                    .map(|found| Underline {
                        line: found.get("range").get("start").get("line").as_index().unwrap_or(0),
                        from: found
                            .get("range")
                            .get("start")
                            .get("character")
                            .as_index()
                            .unwrap_or(0),
                        to: found
                            .get("range")
                            .get("end")
                            .get("character")
                            .as_index()
                            .unwrap_or(0),
                        message: found.get("message").as_str().unwrap_or("").to_string(),
                    })
                    .collect(),
            );
        }
        found
    }

    /// The last thing the editor was told about this URI.
    fn last(&self, uri: &str) -> Option<Vec<Underline>> {
        self.publications(uri).pop()
    }

    /// The answer to the request with this id, as a parsed message.
    fn answer(&self, id: i64) -> Option<wire::Value> {
        self.0.iter().rev().find_map(|body| {
            let message = wire::parse(body)?;
            (message.get("id").as_i64() == Some(id) && message.get("method").is_null())
                .then_some(message)
        })
    }

    fn answered(&self, id: i64) -> bool {
        self.answer(id).is_some()
    }
}

const PAGE: &str = "file:///c:/project/app/page.rsx";

/// A page that compiles.
const GOOD: &str = "fn Page() -> Element {\n    return <p>bonjour</p>;\n}\n";
/// The same page with a tag the compiler refuses, on its second line.
const BAD: &str = "fn Page() -> Element {\n    return <dvi>bonjour</dvi>;\n}\n";

// ---------------------------------------------------------------- handshake

#[test]
fn the_handshake_names_the_server_and_the_sync_it_wants() {
    let said = Session::new()
        .request(1, "initialize", "{}")
        .send(r#"{"jsonrpc":"2.0","method":"exit"}"#)
        .run();
    let answer = said.answer(1).expect("an answer to initialize");
    let capabilities = answer.get("result").get("capabilities");
    assert_eq!(
        capabilities.get("textDocumentSync").as_i64(),
        Some(1),
        "the whole text on every change is what the compiler needs"
    );
    assert!(
        !capabilities.get("completionProvider").is_null(),
        "completion is the server's own and needs nothing installed"
    );
    assert_eq!(
        answer.get("result").get("serverInfo").get("name").as_str(),
        Some("rustx-lsp")
    );
}

#[test]
fn an_id_that_was_sent_as_a_string_comes_back_as_a_string() {
    // The specification allows it, and an editor matches an answer to a
    // question by that id: a string answered with a number is a question
    // that is never answered at all.
    let said = Session::new()
        .send(r#"{"jsonrpc":"2.0","id":"abc","method":"initialize","params":{}}"#)
        .send(r#"{"jsonrpc":"2.0","method":"exit"}"#)
        .run();
    assert!(
        said.0.iter().any(|body| body.contains("\"id\":\"abc\"")),
        "got {:?}",
        said.0
    );
}

#[test]
fn a_request_the_server_does_not_answer_is_refused_rather_than_ignored() {
    // An editor waits on a request until it times out. Silence is the one
    // answer that costs a developer a spinner.
    let said = Session::started()
        .request(42, "textDocument/foldingRange", "{}")
        .ended();
    let answer = said.answer(42).expect("an answer to an unknown request");
    assert_eq!(answer.get("error").get("code").as_i64(), Some(-32601));
    assert!(said.answered(9999), "the session went on");
}

#[test]
fn a_notification_the_server_does_not_know_is_ignored_in_silence() {
    let said = Session::started()
        .send(r#"{"jsonrpc":"2.0","method":"workspace/didChangeWatchedFiles","params":{}}"#)
        .ended();
    assert!(said.answered(9999));
    assert!(
        !said.0.iter().any(|body| body.contains("-32601")),
        "a notification must not be refused: {:?}",
        said.0
    );
}

// ------------------------------------------------------------- diagnostics

#[test]
fn a_page_that_does_not_compile_is_underlined_at_the_exact_word() {
    let said = Session::started().open(PAGE, BAD).ended();
    let found = said.last(PAGE).expect("a publication");
    assert_eq!(found.len(), 1, "got {found:?}");
    // `    return <dvi>` — the second line, and `dvi` starts at the
    // twelfth character counted from zero.
    assert_eq!(found[0].line, 1);
    // The compiler points at the `<` that opens the tag, which is the
    // eleventh character counted from zero, and the underline covers
    // `<dvi>` — not the rest of the line, which is what it used to do.
    assert_eq!(found[0].from, 11, "got {found:?}");
    assert_eq!(found[0].to, 16, "the underline covers `<dvi>` and no more");
    assert!(found[0].message.contains("dvi"), "got {found:?}");
}

#[test]
fn a_page_that_compiles_is_told_so_rather_than_left_alone() {
    // Silence leaves the last underline on screen after the mistake has
    // been fixed.
    let said = Session::started().open(PAGE, GOOD).ended();
    assert_eq!(said.last(PAGE), Some(Vec::new()));
}

#[test]
fn a_mistake_that_is_fixed_leaves_nothing_behind() {
    let said = Session::started().open(PAGE, BAD).replace(PAGE, GOOD).ended();
    let publications = said.publications(PAGE);
    assert_eq!(publications.len(), 2, "got {publications:?}");
    assert_eq!(publications[0].len(), 1);
    assert_eq!(publications[1], Vec::new(), "the underline was withdrawn");
}

#[test]
fn the_underline_follows_the_text_when_the_mistake_moves() {
    // A blank line inserted at the top moves every line under it. A
    // server whose text had drifted from the editor's would publish the
    // old line number, and the underline would sit a line above the
    // word.
    let said = Session::started()
        .open(PAGE, BAD)
        .edit(PAGE, (0, 0), (0, 0), "\n")
        .ended();
    let found = said.last(PAGE).expect("a publication");
    assert_eq!(found.len(), 1, "got {found:?}");
    assert_eq!(found[0].line, 2, "the mistake moved down a line");
    assert_eq!(found[0].from, 11);
}

#[test]
fn a_column_is_published_in_the_units_an_editor_counts_in() {
    // LSP counts characters in UTF-16 code units. `😀` is one Rust
    // character and two of those, so a mistake after one is two columns
    // further along than the compiler thinks — and the underline used to
    // be drawn one letter to the left.
    let source = "fn Page() -> Element {\n    return <p>😀<dvi>x</dvi></p>;\n}\n";
    let said = Session::started().open(PAGE, source).ended();
    let found = said.last(PAGE).expect("a publication");
    assert_eq!(found.len(), 1, "got {found:?}");
    let line = source.lines().nth(1).expect("the second line");
    let expected: usize = line
        .char_indices()
        .find(|(_, letter)| *letter == 'd')
        .map(|(at, _)| line[..at - 1].encode_utf16().count())
        .expect("the refused tag");
    assert_eq!(
        found[0].from, expected,
        "the underline is at the `<` of `<dvi>`, counted in UTF-16 units"
    );
}

// ------------------------------------------------------- buffer and server

#[test]
fn a_document_whose_text_holds_the_name_of_a_field_is_still_read_whole() {
    // The defect this is a regression test for: a field was found by
    // searching the message for its quoted name, and a page is allowed
    // to contain `"text":` and `"uri":`. The document became whatever
    // the search found first.
    let source = concat!(
        "fn Page() -> Element {\n",
        "    let json = \"{\\\"uri\\\":\\\"nowhere\\\",\\\"text\\\":\\\"not the page\\\"}\";\n",
        "    return <dvi>{json}</dvi>;\n",
        "}\n"
    );
    let said = Session::started().open(PAGE, source).ended();
    let found = said.last(PAGE).expect("a publication");
    assert_eq!(found.len(), 1, "got {found:?}");
    // On the third line, which is where the refused tag is — and not on
    // the first, which is where it would be if the page had been
    // truncated to a fragment.
    assert_eq!(found[0].line, 2, "got {found:?}");
}

#[test]
fn an_emoji_sent_as_a_surrogate_pair_keeps_every_position_after_it() {
    // A client may write any character as `\uXXXX`, and anything outside
    // the basic plane as two of them. Decoded apart, both halves are
    // invalid and both used to be dropped — so the document was two
    // characters shorter than the editor's and every line after it was
    // out.
    let escaped = concat!(
        "fn Page() -> Element {\\n",
        "    let face = \\\"\\ud83d\\ude00\\\";\\n",
        "    return <dvi>{face}</dvi>;\\n",
        "}\\n"
    );
    let body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{PAGE}","languageId":"rsx","version":1,"text":"{escaped}"}}}}}}"#
    );
    let said = Session::started().send(&body).ended();
    let found = said.last(PAGE).expect("a publication");
    assert_eq!(found.len(), 1, "got {found:?}");
    assert_eq!(found[0].line, 2, "got {found:?}");
}

#[test]
fn a_client_that_sends_ranges_is_applied_and_not_misread() {
    // This server asks for whole texts. A client that sends ranges
    // anyway used to have its three characters taken for the whole
    // document — and then every diagnostic for the rest of the session
    // was about those three characters.
    let said = Session::started()
        .open(PAGE, GOOD)
        // `<p>` becomes `<dvi>`: replace the one character `p`.
        .edit(PAGE, (1, 12), (1, 13), "dvi")
        .edit(PAGE, (1, 25), (1, 26), "dvi")
        .ended();
    let found = said.last(PAGE).expect("a publication");
    assert_eq!(found.len(), 1, "got {found:?}");
    assert_eq!(found[0].line, 1, "got {found:?}");
    assert!(found[0].message.contains("dvi"), "got {found:?}");
}

#[test]
fn a_change_for_a_document_nobody_opened_publishes_nothing() {
    let said = Session::started().replace(PAGE, BAD).ended();
    assert!(
        said.publications(PAGE).is_empty(),
        "got {:?}",
        said.publications(PAGE)
    );
    assert!(said.answered(9999), "the session went on");
}

#[test]
fn a_closed_document_has_its_underlines_withdrawn() {
    // An editor shows what a server published until the server says
    // there is nothing. A closed file with a red squiggle still in the
    // problems panel is a lie that survives until the window is
    // reloaded.
    let said = Session::started().open(PAGE, BAD).close(PAGE).ended();
    let publications = said.publications(PAGE);
    assert_eq!(publications.len(), 2, "got {publications:?}");
    assert_eq!(publications[0].len(), 1);
    assert_eq!(publications[1], Vec::new(), "nothing is left on screen");
}

#[test]
fn a_change_after_a_close_is_ignored_rather_than_reopening_the_file() {
    let said = Session::started()
        .open(PAGE, GOOD)
        .close(PAGE)
        .replace(PAGE, BAD)
        .ended();
    let publications = said.publications(PAGE);
    assert_eq!(
        publications.len(),
        2,
        "the open and the close, and nothing after them: {publications:?}"
    );
    assert_eq!(publications[1], Vec::new());
}

#[test]
fn a_document_closed_and_opened_again_is_read_from_its_first_byte() {
    let said = Session::started()
        .open(PAGE, BAD)
        .close(PAGE)
        .open(PAGE, GOOD)
        .ended();
    assert_eq!(said.last(PAGE), Some(Vec::new()));
}

#[test]
fn two_documents_do_not_share_their_underlines() {
    let other = "file:///c:/project/app/other.rsx";
    let said = Session::started()
        .open(PAGE, BAD)
        .open(other, GOOD)
        .ended();
    assert_eq!(said.last(PAGE).map(|found| found.len()), Some(1));
    assert_eq!(said.last(other), Some(Vec::new()));
}

// ------------------------------------------------------------ broken input

#[test]
fn a_malformed_frame_does_not_end_the_session() {
    // The defect: a frame with no `Content-Length` answered the same as
    // a closed pipe, so one malformed message and the editor lost every
    // underline with no message saying why.
    let said = Session::started()
        .send_raw(b"Content-Type: application/vscode-jsonrpc\r\n\r\n")
        .open(PAGE, BAD)
        .ended();
    assert_eq!(said.last(PAGE).map(|found| found.len()), Some(1));
    assert!(said.answered(9999), "the session went on");
}

#[test]
fn a_body_that_is_not_utf8_is_skipped_and_the_session_goes_on() {
    let mut session = Session::started();
    session.send_raw(b"Content-Length: 3\r\n\r\n");
    session.send_raw(&[0xff, 0xfe, 0xfd]);
    let said = session.open(PAGE, BAD).ended();
    assert_eq!(said.last(PAGE).map(|found| found.len()), Some(1));
    assert!(said.answered(9999));
}

#[test]
fn a_body_that_is_not_json_is_skipped_and_the_session_goes_on() {
    let said = Session::started()
        .send("this is not json at all")
        .send("{\"jsonrpc\":\"2.0\"")
        .open(PAGE, BAD)
        .ended();
    assert_eq!(said.last(PAGE).map(|found| found.len()), Some(1));
    assert!(said.answered(9999));
}

#[test]
fn a_length_past_all_reason_is_refused_rather_than_allocated() {
    // `vec![0; length]` on a header nobody checked is an abort, not a
    // failure: no `catch_unwind` recovers from an allocation the
    // operating system refused.
    let said = Session::started()
        .send_raw(b"Content-Length: 99999999999999\r\n\r\n")
        .open(PAGE, GOOD)
        .ended();
    assert_eq!(said.last(PAGE), Some(Vec::new()));
}

#[test]
fn a_position_the_editor_could_not_have_meant_is_answered_rather_than_crashed() {
    let said = Session::started()
        .open(PAGE, GOOD)
        .request(
            5,
            "textDocument/completion",
            &format!(
                r#"{{"textDocument":{{"uri":"{PAGE}"}},"position":{{"line":99999,"character":99999}}}}"#
            ),
        )
        .request(
            6,
            "textDocument/completion",
            &format!(
                r#"{{"textDocument":{{"uri":"{PAGE}"}},"position":{{"line":-4,"character":-9}}}}"#
            ),
        )
        .request(7, "textDocument/completion", r#"{"textDocument":{}}"#)
        .ended();
    for id in [5, 6, 7] {
        assert!(said.answered(id), "nothing came back for {id}");
    }
    assert!(said.answered(9999));
}

#[test]
fn a_question_about_a_document_that_was_never_opened_is_answered_with_nothing() {
    let said = Session::started()
        .request(
            5,
            "textDocument/hover",
            &format!(
                r#"{{"textDocument":{{"uri":"{PAGE}"}},"position":{{"line":0,"character":0}}}}"#
            ),
        )
        .ended();
    let answer = said.answer(5).expect("an answer");
    assert!(
        answer.get("result").is_null(),
        "a hover with nothing to say is null: {:?}",
        answer
    );
    assert!(answer.get("error").is_null(), "and never an error");
}

// ------------------------------------------------- typing, and recovery

/// Every prefix of a text, which is what a file looks like while
/// somebody types it.
fn typed_out(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut so_far = String::new();
    for letter in text.chars() {
        so_far.push(letter);
        found.push(so_far.clone());
    }
    found
}

#[test]
fn a_page_typed_one_character_at_a_time_ends_with_no_underline_and_a_live_server() {
    // §5 of the stability mission, named outright: invalid `.rsx` syntax
    // followed by recovery during editing. Every intermediate state here
    // is a real one — half a tag, an attribute with no value, a `{` with
    // nothing in it — and the compiler is called on each. What must hold
    // at the end: the server is still answering, and the one thing the
    // editor is still showing is nothing.
    let mut session = Session::started();
    session.open(PAGE, "");
    for state in typed_out(GOOD) {
        session.replace(PAGE, &state);
    }
    let said = session.ended();

    assert!(
        said.answered(9999),
        "the server did not survive being typed into"
    );
    assert_eq!(
        said.last(PAGE),
        Some(Vec::new()),
        "the page compiles now, so nothing should be underlined"
    );
}

#[test]
fn the_same_through_a_tag_that_is_broken_and_then_mended() {
    // Typed, broken, mended: `<p>` becomes `<dvi>` becomes `<p>` again,
    // one character at a time, with a range change for each of them —
    // which is the sequence that drifts if the document store is wrong.
    let mut session = Session::started();
    session.open(PAGE, GOOD);
    // `p` → `pz` → `pzz`, all refused, then back.
    session.edit(PAGE, (1, 13), (1, 13), "z");
    session.edit(PAGE, (1, 14), (1, 14), "z");
    session.edit(PAGE, (1, 13), (1, 15), "");
    let said = session.ended();

    let publications = said.publications(PAGE);
    assert!(publications.len() >= 2, "got {publications:?}");
    assert_eq!(
        publications.last(),
        Some(&Vec::new()),
        "the tag is `<p>` again: {publications:?}"
    );
    assert!(said.answered(9999));
}

#[test]
fn a_page_deleted_down_to_nothing_and_typed_again_leaves_nothing_behind() {
    let mut session = Session::started();
    session.open(PAGE, GOOD);
    // Select all, delete.
    session.edit(PAGE, (0, 0), (3, 0), "");
    session.replace(PAGE, GOOD);
    let said = session.ended();
    assert_eq!(said.last(PAGE), Some(Vec::new()));
    assert!(said.answered(9999));
}

#[test]
fn every_half_typed_shape_a_rsx_passes_through_is_survived() {
    let half = [
        "fn Page() -> Element {\n    return <",
        "fn Page() -> Element {\n    return </",
        "fn Page() -> Element {\n    return <p class=",
        "fn Page() -> Element {\n    return <p class=\"",
        "fn Page() -> Element {\n    return <p style={{",
        "fn Page() -> Element {\n    return <p>{",
        "fn Page() -> Element {\n    return <p>{let mut",
        "fn Page() -> Element {\n    return <p>{n",
        "fn Page() -> Element {\n    return <p attribute=\"",
        "fn Page() -> Element {\n    let v: Vec<",
        "fn Page() -> Element {\n    if a < b {",
        "",
        "\n\n\n",
        "}",
        "<",
        "{",
        "\"",
    ];
    let mut session = Session::started();
    session.open(PAGE, "");
    for state in half {
        session.replace(PAGE, state);
        // And a question at the end of it, which is where a developer's
        // cursor is.
        let line = state.lines().count().saturating_sub(1);
        let column = state.lines().next_back().map_or(0, |line| line.chars().count());
        session.request(
            1000 + line as i64,
            "textDocument/completion",
            &format!(
                r#"{{"textDocument":{{"uri":"{PAGE}"}},"position":{{"line":{line},"character":{column}}}}}"#
            ),
        );
    }
    let said = session.ended();
    assert!(
        said.answered(9999),
        "the server did not survive a half-typed page"
    );
}

#[test]
fn a_page_full_of_characters_outside_the_basic_plane_is_survived() {
    let awkward = [
        "fn Page() -> Element {\n    return <p>😀🇫🇷👨‍👩‍👧‍👦</p>;\n}\n",
        "fn Page() -> Element {\n    return <p>e\u{301}\u{302}\u{303}</p>;\n}\n",
        "fn Page() -> Element {\r\n    return <p>déjà</p>;\r\n}\r\n",
        "fn Page() -> Element {\n    return <p>\u{200b}\u{feff}</p>;\n}\n",
        "fn Page() -> Element {\n    return <p>\u{10FFFF}</p>;\n}\n",
    ];
    let mut session = Session::started();
    session.open(PAGE, "");
    for (which, state) in awkward.iter().enumerate() {
        session.replace(PAGE, state);
        session.request(
            2000 + which as i64,
            "textDocument/completion",
            &format!(
                r#"{{"textDocument":{{"uri":"{PAGE}"}},"position":{{"line":1,"character":15}}}}"#
            ),
        );
    }
    let said = session.ended();
    assert!(said.answered(9999));
    for which in 0..awkward.len() {
        assert!(
            said.answered(2000 + which as i64),
            "nothing came back for the {which}th"
        );
    }
}

// ----------------------------------------------------------- reactivity

#[test]
fn a_burst_of_keystrokes_costs_one_compile_and_the_last_word() {
    // Twenty changes already in the pipe: nineteen of them describe a
    // text that is no longer on screen. Every one is applied, and only
    // the last is compiled — and what goes out is about the last.
    let mut session = Session::started();
    session.open(PAGE, GOOD);
    let states: Vec<String> = (0..20)
        .map(|at| format!("fn Page() -> Element {{\n    return <p>{at}</p>;\n}}\n"))
        .collect();
    for state in &states {
        session.replace(PAGE, state);
    }
    session.replace(PAGE, BAD);
    let said = session.ended();

    let publications = said.publications(PAGE);
    assert!(
        publications.len() <= 3,
        "a burst of twenty-one changes was compiled {} times",
        publications.len()
    );
    assert_eq!(
        publications.last().map(|found| found.len()),
        Some(1),
        "the last text is the one that was underlined: {publications:?}"
    );
}

#[test]
fn a_burst_is_never_allowed_to_swallow_the_last_word() {
    // The danger of coalescing: an underline that is skipped and never
    // sent. Whatever else happens, the editor must end up holding the
    // truth about the final text.
    for last in [GOOD, BAD] {
        let mut session = Session::started();
        session.open(PAGE, GOOD);
        for at in 0..30 {
            session.replace(
                PAGE,
                &format!("fn Page() -> Element {{\n    return <p>{at}</p>;\n}}\n"),
            );
        }
        session.replace(PAGE, last);
        let said = session.ended();
        let expected = if std::ptr::eq(last, GOOD) { 0 } else { 1 };
        assert_eq!(
            said.last(PAGE).map(|found| found.len()),
            Some(expected),
            "the final text was not the one published"
        );
    }
}

#[test]
fn a_question_in_the_middle_of_a_burst_is_answered_about_the_text_as_it_is() {
    let mut session = Session::started();
    session.open(PAGE, GOOD);
    for at in 0..10 {
        session.replace(
            PAGE,
            &format!("fn Page() -> Element {{\n    return <p>{at}</p>;\n}}\n"),
        );
    }
    session.request(
        77,
        "textDocument/completion",
        &format!(
            r#"{{"textDocument":{{"uri":"{PAGE}"}},"position":{{"line":1,"character":12}}}}"#
        ),
    );
    let said = session.ended();
    assert!(said.answered(77), "the question was answered");
    // And the underlining was not left waiting behind the question.
    assert_eq!(said.last(PAGE), Some(Vec::new()));
}

#[test]
fn a_five_thousand_line_page_is_answered_in_a_time_a_keystroke_can_hold() {
    let mut long = String::from("fn Page() -> Element {\n    return <ul>\n");
    for at in 0..5000 {
        long.push_str(&format!("        <li>row {at}</li>\n"));
    }
    long.push_str("    </ul>;\n}\n");

    let mut session = Session::new();
    session.open(PAGE, &long);
    let started = std::time::Instant::now();
    let said = session.ended();
    let took = started.elapsed();
    assert_eq!(said.last(PAGE), Some(Vec::new()));
    let ceiling = if cfg!(debug_assertions) { 2_000 } else { 200 };
    assert!(
        took.as_millis() < ceiling,
        "a five thousand line page took {}ms",
        took.as_millis()
    );
}

#[test]
fn opening_and_closing_many_documents_does_not_leave_them_behind() {
    // What the server holds is bounded by what the editor has open. The
    // proof available from outside is that a question about a closed
    // document is answered as a question about nothing, however many
    // were opened before it.
    let mut session = Session::started();
    for at in 0..200 {
        let uri = format!("file:///c:/project/app/page{at}.rsx");
        session.open(&uri, BAD);
        session.close(&uri);
    }
    session.request(
        50,
        "textDocument/completion",
        r#"{"textDocument":{"uri":"file:///c:/project/app/page7.rsx"},"position":{"line":1,"character":13}}"#,
    );
    let said = session.ended();
    let answer = said.answer(50).expect("an answer");
    let items = answer.get("result").get("items");
    assert!(
        items.items().is_empty(),
        "a closed document still had a text: {:?}",
        answer
    );
    for at in 0..200 {
        let uri = format!("file:///c:/project/app/page{at}.rsx");
        assert_eq!(
            said.last(&uri),
            Some(Vec::new()),
            "page{at} kept its underline after being closed"
        );
    }
}

#[test]
fn shutdown_is_answered_and_nothing_after_exit_is() {
    let said = Session::new()
        .request(9, "shutdown", "null")
        .send(r#"{"jsonrpc":"2.0","method":"exit"}"#)
        .request(10, "initialize", "{}")
        .run();
    assert!(said.answered(9));
    assert!(
        !said.answered(10),
        "the server answered after exit: {:?}",
        said.0
    );
}
