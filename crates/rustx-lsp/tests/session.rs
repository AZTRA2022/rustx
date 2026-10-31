//! A session: what an editor sends, and what comes back.
//!
//! Driven through the server's own loop with a stream in and a stream
//! out, which is all a language server is. No editor is needed to know
//! whether the right line is underlined.

use rustx_lsp::{json, serve};

/// The messages a client would send, framed.
fn conversation(messages: &[&str]) -> Vec<u8> {
    let mut stream = Vec::new();
    for body in messages {
        stream
            .extend_from_slice(format!("Content-Length: {}\r\n\r\n{body}", body.len()).as_bytes());
    }
    stream
}

/// Everything the server said, split into bodies.
fn answers(given: &[&str]) -> Vec<String> {
    let input = conversation(given);
    let mut output: Vec<u8> = Vec::new();
    serve(&mut std::io::Cursor::new(input), &mut output);
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

fn opened(text: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{{\
         \"textDocument\":{{\"uri\":\"file:///c:/app/page.rsx\",\"languageId\":\"rsx\",\
         \"version\":1,\"text\":\"{}\"}}}}}}",
        json::escape(text)
    )
}

const GOOD: &str = "fn Page() -> Element {\n    return <p>bonjour</p>;\n}\n";
const BAD: &str = "fn Page() -> Element {\n    return <dvi>bonjour</dvi>;\n}\n";

#[test]
fn initialize_answers_with_full_text_sync() {
    let said = answers(&["{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}"]);
    assert_eq!(said.len(), 1, "got {said:?}");
    assert!(said[0].contains("\"id\":1"), "got {}", said[0]);
    // 1 is the whole text on every change, which is what a compiler
    // that reads a file from its first byte needs.
    assert!(
        said[0].contains("\"textDocumentSync\":1"),
        "got {}",
        said[0]
    );
}

#[test]
fn a_page_that_does_not_compile_is_reported_at_its_line() {
    let said = answers(&[opened(BAD).as_str()]);
    let published = said
        .iter()
        .find(|body| body.contains("publishDiagnostics"))
        .unwrap_or_else(|| panic!("nothing was published: {said:?}"));
    // The page's own coordinates, counted from zero as LSP counts them:
    // `<dvi>` is on the second line.
    assert!(published.contains("\"line\":1"), "got {published}");
    assert!(published.contains("dvi"), "got {published}");
    assert!(
        published.contains("file:///c:/app/page.rsx"),
        "got {published}"
    );
}

#[test]
fn a_page_that_compiles_publishes_an_empty_list() {
    // An empty list, not silence: silence leaves the last underline on
    // screen after the mistake has been fixed.
    let said = answers(&[opened(GOOD).as_str()]);
    let published = said
        .iter()
        .find(|body| body.contains("publishDiagnostics"))
        .unwrap_or_else(|| panic!("nothing was published: {said:?}"));
    assert!(published.contains("\"diagnostics\":[]"), "got {published}");
}

#[test]
fn a_change_republishes() {
    let changed = format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didChange\",\"params\":{{\
         \"textDocument\":{{\"uri\":\"file:///c:/app/page.rsx\",\"version\":2}},\
         \"contentChanges\":[{{\"text\":\"{}\"}}]}}}}",
        json::escape(BAD)
    );
    let said = answers(&[opened(GOOD).as_str(), changed.as_str()]);
    let published: Vec<&String> = said
        .iter()
        .filter(|body| body.contains("publishDiagnostics"))
        .collect();
    assert_eq!(published.len(), 2, "got {said:?}");
    assert!(
        published[0].contains("\"diagnostics\":[]"),
        "got {}",
        published[0]
    );
    assert!(published[1].contains("dvi"), "got {}", published[1]);
}

#[test]
fn shutdown_then_exit_stops_the_server() {
    let said = answers(&[
        "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"shutdown\",\"params\":null}",
        "{\"jsonrpc\":\"2.0\",\"method\":\"exit\"}",
        // Anything after `exit` must not be answered: VS Code starts one
        // server per window and leaves it to close itself.
        "{\"jsonrpc\":\"2.0\",\"id\":10,\"method\":\"initialize\",\"params\":{}}",
    ]);
    assert_eq!(said.len(), 1, "got {said:?}");
    assert!(said[0].contains("\"id\":9"), "got {}", said[0]);
}

#[test]
fn a_long_page_is_compiled_in_a_time_a_keystroke_can_hold() {
    // Five thousand lines, compiled on every keystroke. A server that
    // takes a second holds the editor hostage.
    let mut long = String::from("fn Page() -> Element {\n    return <ul>\n");
    for at in 0..5000 {
        long.push_str(&format!("        <li>row {at}</li>\n"));
    }
    long.push_str("    </ul>;\n}\n");

    let started = std::time::Instant::now();
    let said = answers(&[opened(&long).as_str()]);
    let took = started.elapsed();
    assert!(
        said.iter().any(|body| body.contains("publishDiagnostics")),
        "nothing was published for a long page"
    );
    let ceiling = if cfg!(debug_assertions) { 2_000 } else { 200 };
    assert!(
        took.as_millis() < ceiling,
        "a five thousand line page took {}ms to answer",
        took.as_millis()
    );
}

#[test]
fn a_half_typed_page_publishes_nothing_and_the_server_lives() {
    // What a `.rsx` looks like most of the time: a tag that has only
    // been half typed. Whatever the compiler does with it — refuse it or
    // panic on it — the server must answer the next message, because an
    // editor whose server dies mid-word underlines nothing ever again.
    let half: Vec<String> = [
        "fn Page() -> Element {\n    return <",
        "fn Page() -> Element {\n    return <p class=",
        "fn Page() -> Element {\n    return <p>{",
        "fn Page() -> Element {\n    return <p>{let mut",
        "fn Page() -> Element {\n    return </",
        "fn Page() -> Element {\n    return <p attribute=\"",
        "",
    ]
    .iter()
    .map(|text| opened(text))
    .collect();

    let mut given: Vec<&str> = half.iter().map(String::as_str).collect();
    given.push("{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"shutdown\",\"params\":null}");

    let said = answers(&given);
    assert!(
        said.last().is_some_and(|body| body.contains("\"id\":7")),
        "the server did not survive a half-typed page: {said:?}"
    );
}
