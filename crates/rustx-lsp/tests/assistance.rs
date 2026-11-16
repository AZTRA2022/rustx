//! Completion, hovers and definitions in a `.rsx`.
//!
//! The server answers them by asking `rust-analyzer` about the shadow
//! file and translating the positions on the way in and on the way out.
//! No `rust-analyzer` is needed to test that: the downstream server is
//! handed in, with its answers written in advance.

use std::io::Cursor;

use rustx_lsp::downstream::Server;
use rustx_lsp::{json, serve_with};

fn framed(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

/// A downstream server whose answers are written in advance, in the
/// order its questions will be asked.
fn canned(answers: &[&str]) -> Server {
    let stream: String = answers.iter().map(|body| framed(body)).collect();
    Server::over(
        Box::new(Cursor::new(stream.into_bytes())),
        Box::new(Vec::new()),
    )
}

/// Everything the server said to the editor, body by body.
fn conversation(given: &[&str], downstream: Option<Server>) -> Vec<String> {
    let stream: String = given.iter().map(|body| framed(body)).collect();
    let mut output: Vec<u8> = Vec::new();
    serve_with(
        &mut Cursor::new(stream.into_bytes()),
        &mut output,
        downstream,
    );
    let said = String::from_utf8(output).expect("utf-8");

    let mut bodies = Vec::new();
    let mut rest = said.as_str();
    while let Some(at) = rest.find("Content-Length: ") {
        let after = &rest[at + "Content-Length: ".len()..];
        let (length, remainder) = after.split_once("\r\n\r\n").expect("a header");
        let length: usize = length.trim().parse().expect("a number");
        bodies.push(remainder[..length].to_string());
        rest = &remainder[length..];
    }
    bodies
}

const PAGE: &str =
    "fn Page() -> Element {\n    let title = \"bonjour\";\n    return <p>{title}</p>;\n}\n";

/// A page of a real application, so that it has a shadow.
fn a_page() -> (std::path::PathBuf, String) {
    let root = std::env::temp_dir().join(format!(
        "rustx-assistance-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(root.join("app")).expect("an app directory");
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"asked\"\n").expect("a manifest");
    let page = root.join("app").join("page.rsx");
    std::fs::write(&page, PAGE).expect("a page");
    let uri = rustx_lsp::shadow::uri_of(&page);
    (root, uri)
}

fn opened(uri: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{{\
         \"textDocument\":{{\"uri\":\"{uri}\",\"languageId\":\"rsx\",\"version\":1,\
         \"text\":\"{}\"}}}}}}",
        json::escape(PAGE)
    )
}

fn asked(method: &str, uri: &str, line: usize, character: usize, id: usize) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"{method}\",\"params\":{{\
         \"textDocument\":{{\"uri\":\"{uri}\"}},\
         \"position\":{{\"line\":{line},\"character\":{character}}}}}}}"
    )
}

#[test]
fn capabilities_do_not_promise_what_there_is_no_server_for() {
    // Hovers and definitions come from `rust-analyzer`; completion does
    // not have to, because the markup's half is the server's own.
    let said = conversation(
        &["{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}"],
        None,
    );
    assert!(!said[0].contains("hoverProvider"), "got {}", said[0]);
    assert!(!said[0].contains("definitionProvider"), "got {}", said[0]);
    // What it does without one is what it has always done, plus the
    // completions that are its own.
    assert!(said[0].contains("textDocumentSync"), "got {}", said[0]);
    assert!(said[0].contains("completionProvider"), "got {}", said[0]);
}

#[test]
fn capabilities_promise_completion_when_there_is_a_server() {
    let said = conversation(
        &["{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}"],
        Some(canned(&[])),
    );
    assert!(said[0].contains("completionProvider"), "got {}", said[0]);
    assert!(said[0].contains("hoverProvider"), "got {}", said[0]);
    assert!(said[0].contains("definitionProvider"), "got {}", said[0]);
}

#[test]
fn a_hover_in_a_component_body_comes_back_with_what_rust_said() {
    let (root, uri) = a_page();
    // Line 2, `title`: ordinary Rust, a column that crosses.
    let said = conversation(
        &[
            opened(&uri).as_str(),
            asked("textDocument/hover", &uri, 1, 9, 7).as_str(),
        ],
        Some(canned(&[
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"contents\":\"&str\"}}",
        ])),
    );
    let answer = said
        .iter()
        .find(|body| body.contains("\"id\":7"))
        .unwrap_or_else(|| panic!("the hover was not answered: {said:?}"));
    assert!(answer.contains("&str"), "got {answer}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_completion_inside_an_expression_comes_back_from_rust() {
    // `{title}` on line 3. A column inside a region has no counterpart
    // in the generated file — the whole region is one line there — so
    // the question is asked against a file built for it, with the
    // expression at its own line and column. The answer is the real one.
    let (root, uri) = a_page();
    let said = conversation(
        &[
            opened(&uri).as_str(),
            asked("textDocument/completion", &uri, 2, 16, 8).as_str(),
        ],
        Some(canned(&[
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"items\":[{\"label\":\"wrong\"}]}}",
        ])),
    );
    let answer = said
        .iter()
        .find(|body| body.contains("\"id\":8"))
        .unwrap_or_else(|| panic!("the completion was not answered: {said:?}"));
    assert!(
        answer.contains("wrong"),
        "the expression's own completions never came back: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_definition_in_the_shadow_comes_back_on_the_rsx() {
    // Otherwise the editor opens a file in `%TEMP%` the developer never
    // wrote.
    let (root, uri) = a_page();
    let shadow = rustx_lsp::shadow::of(&uri, PAGE).expect("a shadow");
    let answer = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":[{{\"uri\":\"{}\",\
         \"range\":{{\"start\":{{\"line\":1,\"character\":8}},\
         \"end\":{{\"line\":1,\"character\":13}}}}}}]}}",
        json::escape(&shadow.uri)
    );
    let said = conversation(
        &[
            opened(&uri).as_str(),
            asked("textDocument/definition", &uri, 1, 9, 9).as_str(),
        ],
        Some(canned(&[answer.as_str()])),
    );
    let given = said
        .iter()
        .find(|body| body.contains("\"id\":9"))
        .unwrap_or_else(|| panic!("the definition was not answered: {said:?}"));
    assert!(given.contains("page.rsx"), "got {given}");
    assert!(!given.contains("page.rs\""), "the shadow leaked: {given}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_definition_in_another_crate_passes_through() {
    // `rustx_dom::Element` is a real file in a real crate, and the
    // editor should open it.
    let (root, uri) = a_page();
    let said = conversation(
        &[
            opened(&uri).as_str(),
            asked("textDocument/definition", &uri, 0, 4, 10).as_str(),
        ],
        Some(canned(&[
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":[{\"uri\":\"file:///c:/rustx/crates/rustx-dom/src/element.rs\",\"range\":{\"start\":{\"line\":4,\"character\":0},\"end\":{\"line\":4,\"character\":7}}}]}",
        ])),
    );
    let given = said
        .iter()
        .find(|body| body.contains("\"id\":10"))
        .unwrap_or_else(|| panic!("not answered: {said:?}"));
    assert!(given.contains("element.rs"), "got {given}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_question_about_a_page_that_was_never_opened_answers_nothing() {
    let said = conversation(
        &[asked(
            "textDocument/hover",
            "file:///c:/nowhere/page.rsx",
            1,
            1,
            11,
        )
        .as_str()],
        Some(canned(&[])),
    );
    assert!(
        said.iter().any(|body| body.contains("\"id\":11")),
        "a question must always be answered: {said:?}"
    );
}

#[test]
fn a_hover_inside_a_tag_answers_nothing() {
    // The markup half answers completion — a tag name, an attribute —
    // and nothing else: there is no type to show for a `<p>`, and an
    // answer taken from the position next door would be about something
    // the developer is not looking at.
    let (root, uri) = a_page();
    let said = conversation(
        &[
            opened(&uri).as_str(),
            asked("textDocument/hover", &uri, 2, 12, 9).as_str(),
        ],
        Some(canned(&[
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"contents\":\"wrong\"}}",
        ])),
    );
    let answer = said
        .iter()
        .find(|body| body.contains("\"id\":9"))
        .unwrap_or_else(|| panic!("the hover was not answered: {said:?}"));
    assert!(!answer.contains("wrong"), "got {answer}");
    assert!(answer.contains("null"), "got {answer}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_trigger_characters_are_where_a_developer_wants_a_list() {
    let said = conversation(
        &["{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}"],
        Some(canned(&[])),
    );
    for character in ["<", "/", "{", "."] {
        assert!(
            said[0].contains(&format!("\\\"{character}\\\"")) || said[0].contains(character),
            "`{character}` does not ask for a list: {}",
            said[0]
        );
    }
}

#[test]
fn completion_is_promised_even_without_a_downstream() {
    // The tags, the attributes and the components need nothing
    // installed. An editor that was not told so would never ask.
    let said = conversation(
        &["{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}"],
        None,
    );
    assert!(said[0].contains("completionProvider"), "got {}", said[0]);
    assert!(!said[0].contains("hoverProvider"), "got {}", said[0]);
}

#[test]
fn a_tag_completes_without_any_downstream() {
    // End to end, through the real server, with nothing behind it: the
    // half that needs nobody answers.
    let (root, uri) = a_page();
    let said = conversation(
        &[
            opened(&uri).as_str(),
            // Line 3, just after the `<` of `<p>`.
            asked("textDocument/completion", &uri, 2, 12, 11).as_str(),
        ],
        None,
    );
    let answer = said
        .iter()
        .find(|body| body.contains("\"id\":11"))
        .unwrap_or_else(|| panic!("the completion was not answered: {said:?}"));
    assert!(
        answer.contains("\"label\":\"p\"") || answer.contains("\"label\":\"path\""),
        "a tag name was not offered: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_completion_rust_was_not_ready_for_is_asked_again() {
    // `rust-analyzer` answers `null` to a question it is not ready for
    // — a workspace still loading, a file it has just been handed — and
    // a developer typing in a page that was opened a second ago is
    // exactly that moment. Asked once, the list is empty for no reason
    // the developer can see; asked twice, it is the real list.
    let (root, uri) = a_page();
    let said = conversation(
        &[
            opened(&uri).as_str(),
            asked("textDocument/completion", &uri, 2, 16, 12).as_str(),
        ],
        Some(canned(&[
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"items\":[{\"label\":\"later\"}]}}",
        ])),
    );
    let answer = said
        .iter()
        .find(|body| body.contains("\"id\":12"))
        .unwrap_or_else(|| panic!("the completion was not answered: {said:?}"));
    assert!(
        answer.contains("later"),
        "the second answer never came back: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A page whose text is given, written where the server can find it.
fn a_page_of(text: &str) -> (std::path::PathBuf, String) {
    let root = std::env::temp_dir().join(format!(
        "rustx-assistance-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(root.join("app")).expect("an app directory");
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"asked\"\n").expect("a manifest");
    let page = root.join("app").join("page.rsx");
    std::fs::write(&page, text).expect("a page");
    let uri = rustx_lsp::shadow::uri_of(&page);
    (root, uri)
}

fn opened_with(uri: &str, text: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{{\
         \"textDocument\":{{\"uri\":\"{uri}\",\"languageId\":\"rsx\",\"version\":1,\
         \"text\":\"{}\"}}}}}}",
        json::escape(text)
    )
}

#[test]
fn a_question_on_a_line_the_rewrite_rebuilt_is_still_answered() {
    // The line reads a value, so the rewrite rebuilt it and the
    // position map calls it opaque -- and a question with no position
    // was answered with `[]`. That is every line that reads a value,
    // every handler, every `async` block: precisely the lines where
    // somebody reaches for a crate's name, which is what made it look
    // as though the whole path had to be typed before anything was
    // offered.
    let page = "fn Page() -> Element {\n    \
                let mut n = 0;\n    \
                let double = n * 2 + rustx::carried::kept();\n    \
                return <p onClick={move |_| n += 1}>{double}</p>;\n}\n";
    let (root, uri) = a_page_of(page);
    let third = page.lines().nth(2).expect("a third line");
    // One past the `t` of `kept`, which is what somebody typing a path
    // has in front of the cursor.
    let character = third.find("kept").expect("the call") + "kept".len();
    let said = conversation(
        &[
            opened_with(&uri, page).as_str(),
            asked("textDocument/completion", &uri, 2, character, 9).as_str(),
        ],
        Some(canned(&[
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"items\":[{\"label\":\"kept\",\
             \"textEdit\":{\"range\":{\"start\":{\"line\":2,\"character\":100},\
             \"end\":{\"line\":2,\"character\":104}},\"newText\":\"kept\"}}]}}",
        ])),
    );
    let answer = said
        .iter()
        .find(|body| body.contains("\"id\":9"))
        .unwrap_or_else(|| panic!("the completion was not answered: {said:?}"));
    assert!(
        answer.contains("kept"),
        "nothing came back on a rewritten line: {answer}"
    );
    // The generated line is six characters wider by here (`n` became
    // `n.get()`), so what comes back has to be six characters narrower
    // than what went out, or the editor would eat the path.
    assert!(
        answer.contains("\"line\":2,\"character\":94"),
        "the range should come back onto the page: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
