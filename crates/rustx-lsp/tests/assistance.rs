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
    let said = conversation(
        &["{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}"],
        None,
    );
    assert!(!said[0].contains("completionProvider"), "got {}", said[0]);
    // What it does without one is what it has always done.
    assert!(said[0].contains("textDocumentSync"), "got {}", said[0]);
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
fn a_completion_inside_a_tag_answers_nothing() {
    // Line 3 from the `<` on is one Rust expression: a column in there
    // has no counterpart, and an answer from a position next door would
    // complete the wrong word.
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
    assert!(!answer.contains("wrong"), "got {answer}");
    assert!(
        answer.contains("[]") || answer.contains("null"),
        "got {answer}"
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
