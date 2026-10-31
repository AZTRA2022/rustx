//! Talking to a language server of our own.
//!
//! What `rustx-lsp` cannot answer — completion, hovers, definitions —
//! it asks `rust-analyzer`. That makes it a client as well as a server,
//! and a client's failures are its own: an answer that belongs to
//! another question, a notification arriving in the middle of one, a
//! child that is not installed, a child that died an hour ago.
//!
//! None of it needs `rust-analyzer` to be tested: a server is two
//! streams, so the tests hand it two.

use std::io::Cursor;
use std::time::{Duration, Instant};

use rustx_lsp::downstream::Server;

fn framed(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

/// A server whose answers are written in advance.
fn canned(messages: &[&str]) -> Server {
    let stream: String = messages.iter().map(|body| framed(body)).collect();
    Server::over(
        Box::new(Cursor::new(stream.into_bytes())),
        Box::new(Vec::new()),
    )
}

#[test]
fn a_question_finds_its_own_answer_among_notifications() {
    // An editor's server talks while it works: progress, logs, the
    // diagnostics of its own. A client that takes the first message for
    // its answer shows a log line as a hover.
    let mut server = canned(&[
        "{\"jsonrpc\":\"2.0\",\"method\":\"window/logMessage\",\"params\":{\"message\":\"indexing\"}}",
        "{\"jsonrpc\":\"2.0\",\"method\":\"$/progress\",\"params\":{}}",
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"contents\":\"u32\"}}",
    ]);
    let answer = server.ask("textDocument/hover", "{}").expect("an answer");
    assert!(answer.contains("u32"), "got {answer}");
}

#[test]
fn an_answer_that_belongs_to_another_question_is_not_taken() {
    // Two questions in flight, answered out of order: each must find its
    // own. The second ask is the one whose id is 2.
    let mut server = canned(&[
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"second\":true}}",
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"first\":true}}",
    ]);
    let answer = server.ask("textDocument/hover", "{}").expect("an answer");
    assert!(answer.contains("first"), "got {answer}");
}

#[test]
fn an_answer_that_never_comes_is_an_empty_answer() {
    // A keystroke must never wait on a server that has stopped
    // answering: what comes back is nothing, and quickly.
    let mut server = canned(&[]);
    let started = Instant::now();
    assert_eq!(server.ask("textDocument/hover", "{}"), None);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "waited {:?}",
        started.elapsed()
    );
}

#[test]
fn a_child_that_is_not_there_is_no_server() {
    // The ordinary case on a fresh machine. Not an error, not a dialog:
    // there is simply no completion, and the underlining carries on.
    assert!(Server::start("rustx-no-such-language-server").is_none());
}

#[test]
fn what_is_asked_is_written_with_an_id_of_its_own() {
    let mut server = canned(&["{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}"]);
    let _ = server.ask("textDocument/definition", "{\"x\":1}");
    let written = server.written();
    assert!(written.contains("\"id\":1"), "got {written}");
    assert!(
        written.contains("\"method\":\"textDocument/definition\""),
        "got {written}"
    );
    assert!(written.contains("Content-Length: "), "got {written}");
}

#[test]
fn a_notification_carries_no_id() {
    // `didOpen` is a notification: an id on it would be a question
    // nobody answers, and the client would wait for the answer.
    let mut server = canned(&[]);
    server.tell("textDocument/didOpen", "{}");
    let written = server.written();
    assert!(!written.contains("\"id\""), "got {written}");
}

#[test]
fn a_server_over_plain_streams_has_nothing_to_restart() {
    // Restarting is for a child, and a child is named by a command. A
    // server over two streams has no command, and says so rather than
    // pretending it started something.
    let mut server = canned(&[]);
    assert!(!server.restart());
}

#[test]
fn a_child_that_answers_initialize_is_held() {
    // The real handshake, without `rust-analyzer`: a child that prints
    // the answer to `initialize` and nothing else. What is proved is
    // that `start` writes the request, matches the answer by its id, and
    // keeps the process.
    let answer = framed("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"capabilities\":{}}}");
    let script = std::env::temp_dir().join(format!(
        "rustx-initialize-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&script, answer.as_bytes()).expect("the answer is written");

    let path = script.display().to_string();
    let server = Server::start_with("cmd", &["/c", "type", &path]).expect("a child that answers");
    assert!(
        server.written().contains("\"method\":\"initialize\""),
        "got {}",
        server.written()
    );
    let _ = std::fs::remove_file(&script);
}

#[test]
fn the_project_the_editor_named_is_the_one_it_is_started_in() {
    // `rust-analyzer` analyses a workspace. Started without one it
    // analyses nothing and answers every question with silence, which
    // looks exactly like a server that was never installed — so the root
    // the editor named has to reach it.
    let answer = framed("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"capabilities\":{}}}");
    let script = std::env::temp_dir().join(format!(
        "rustx-rooted-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&script, answer.as_bytes()).expect("the answer is written");

    let asked = rustx_lsp::downstream::handshake(Some("file:///c:/work/project"), &[]);
    assert!(
        asked.contains("\"rootUri\":\"file:///c:/work/project\""),
        "got {asked}"
    );
    assert!(asked.contains("workspaceFolders"), "got {asked}");

    // And without one, it says so rather than inventing a project.
    let loose = rustx_lsp::downstream::handshake(None, &[]);
    assert!(loose.contains("\"rootUri\":null"), "got {loose}");

    // The child that only prints an answer was told no root, and holds
    // none.
    let path = script.display().to_string();
    let held = Server::start_with("cmd", &["/c", "type", &path]).expect("a child");
    assert!(held.root().is_none(), "nothing named a root");
    let _ = std::fs::remove_file(&script);
}

#[test]
fn the_projects_to_read_are_named_in_the_handshake() {
    // `rust-analyzer` would otherwise look for projects itself and find
    // the application, whose pages are included from inside a macro
    // expansion and so belong to no file it can answer about. Proved by
    // hand against a real `rust-analyzer`: the same page, included from
    // ordinary source, answers.
    let asked = rustx_lsp::downstream::handshake(
        Some("file:///c:/work"),
        &[std::path::PathBuf::from(
            r"C:\work\apps\counter\target\rustx-editor\Cargo.toml",
        )],
    );
    assert!(asked.contains("linkedProjects"), "got {asked}");
    assert!(
        asked.contains("C:/work/apps/counter/target/rustx-editor/Cargo.toml"),
        "a path a server cannot read: {asked}"
    );
}
