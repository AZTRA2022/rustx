//! Every completion item says which page it came from — `data` is not
//! enough, because `completionItem/resolve` is answered about one item
//! with no document attached to the question at all.

use std::io::Cursor;

use rustx_lsp::downstream::Server;
use rustx_lsp::{json, serve_with};

fn framed(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

fn canned(answers: &[&str]) -> Server {
    let stream: String = answers.iter().map(|body| framed(body)).collect();
    Server::over(
        Box::new(Cursor::new(stream.into_bytes())),
        Box::new(Vec::new()),
    )
}

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

fn a_page() -> (std::path::PathBuf, String) {
    let root = std::env::temp_dir().join(format!(
        "rustx-resolve-{}-{}",
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
fn every_completion_item_is_tagged_with_its_page() {
    // Line 3, `{title}`: a region, asked against the file built for it.
    let (root, uri) = a_page();
    let said = conversation(
        &[
            opened(&uri).as_str(),
            asked("textDocument/completion", &uri, 2, 16, 20).as_str(),
        ],
        Some(canned(&[
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"items\":[{\"label\":\"wrong\",\"data\":{\"x\":1}}]}}",
        ])),
    );
    let answer = said
        .iter()
        .find(|body| body.contains("\"id\":20"))
        .unwrap_or_else(|| panic!("the completion was not answered: {said:?}"));
    // The real page's own URI, never the shadow's.
    assert!(
        answer.contains(&format!("\"__rustxUri\":\"{uri}\"")),
        "got {answer}"
    );
    assert!(!answer.contains("page.rs\""), "the shadow leaked: {answer}");
    // What `rust-analyzer` itself put on the item travels unharmed.
    assert!(answer.contains("\"data\":{\"x\":1}"), "got {answer}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_tags_own_completion_is_tagged_too() {
    // The markup half answers out of its own head, with no `data` at
    // all — `completionItem/resolve` must still be well-formed on one
    // of these (Review Focus 2 of the plan).
    let (root, uri) = a_page();
    let said = conversation(
        &[
            opened(&uri).as_str(),
            asked("textDocument/completion", &uri, 2, 12, 21).as_str(),
        ],
        None,
    );
    let answer = said
        .iter()
        .find(|body| body.contains("\"id\":21"))
        .unwrap_or_else(|| panic!("the completion was not answered: {said:?}"));
    assert!(
        answer.contains(&format!("\"__rustxUri\":\"{uri}\"")),
        "got {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_hover_is_never_tagged() {
    // Resolve exists only for completion; tagging a hover would be a
    // promise this server does not make and never answers.
    let (root, uri) = a_page();
    let said = conversation(
        &[
            opened(&uri).as_str(),
            asked("textDocument/hover", &uri, 1, 9, 22).as_str(),
        ],
        Some(canned(&[
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"contents\":\"&str\"}}",
        ])),
    );
    let answer = said
        .iter()
        .find(|body| body.contains("\"id\":22"))
        .unwrap_or_else(|| panic!("the hover was not answered: {said:?}"));
    assert!(!answer.contains("__rustxUri"), "got {answer}");
    let _ = std::fs::remove_dir_all(&root);
}
