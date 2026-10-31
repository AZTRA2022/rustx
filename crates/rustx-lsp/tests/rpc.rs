//! What a language server reads and writes.
//!
//! LSP is JSON with a `Content-Length` in front of it. The reading is
//! where a server goes wrong: a pipe hands over whatever it has, so a
//! header can arrive in two pieces and a body can arrive before the
//! reader has asked for it. A server that reads a line and hopes
//! desynchronises on the first fast typist.

use std::io::Cursor;

use rustx_lsp::{json, rpc};

fn framed(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

#[test]
fn a_message_is_read_with_its_length() {
    let mut input = Cursor::new(framed("{\"id\":1}").into_bytes());
    assert_eq!(rpc::read(&mut input).as_deref(), Some("{\"id\":1}"));
    assert_eq!(rpc::read(&mut input), None, "the stream is over");
}

#[test]
fn two_messages_in_one_stream_are_two_messages() {
    let stream = format!("{}{}", framed("{\"id\":1}"), framed("{\"id\":2}"));
    let mut input = Cursor::new(stream.into_bytes());
    assert_eq!(rpc::read(&mut input).as_deref(), Some("{\"id\":1}"));
    assert_eq!(rpc::read(&mut input).as_deref(), Some("{\"id\":2}"));
}

#[test]
fn a_body_with_a_multibyte_character_is_read_whole() {
    // `Content-Length` counts bytes, and a `.rsx` is full of accented
    // text: reading characters instead of bytes cuts the last one off
    // and every message after it is garbage.
    let body = "{\"text\":\"écran — déjà\"}";
    let mut input = Cursor::new(framed(body).into_bytes());
    assert_eq!(rpc::read(&mut input).as_deref(), Some(body));
}

#[test]
fn what_is_written_carries_its_length_in_bytes() {
    let mut out: Vec<u8> = Vec::new();
    rpc::write(&mut out, "{\"text\":\"déjà\"}");
    let written = String::from_utf8(out).expect("utf-8");
    assert!(
        written.starts_with("Content-Length: 17\r\n\r\n"),
        "got {written:?}"
    );
}

#[test]
fn a_string_with_quotes_and_newlines_survives_escaping() {
    assert_eq!(json::escape("a \"b\"\nc\\d"), "a \\\"b\\\"\\nc\\\\d");
}

#[test]
fn a_field_is_found_wherever_it_sits() {
    let message = "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\
         \"params\":{\"textDocument\":{\"uri\":\"file:///c:/a.rsx\",\"text\":\"fn P() {}\"}}}";
    assert_eq!(
        json::field(message, "method"),
        Some("textDocument/didOpen".to_string())
    );
    assert_eq!(
        json::field(message, "uri"),
        Some("file:///c:/a.rsx".to_string())
    );
    assert_eq!(json::field(message, "text"), Some("fn P() {}".to_string()));
}

#[test]
fn a_field_that_was_escaped_comes_back_as_it_was_written() {
    let message = "{\"text\":\"line\none\t\\\"two\\\"\"}";
    assert_eq!(
        json::field(message, "text"),
        Some("line\none\t\"two\"".to_string())
    );
}

#[test]
fn a_number_field_is_read_as_one() {
    assert_eq!(json::number("{\"id\":42,\"x\":1}", "id"), Some(42));
    assert_eq!(json::number("{\"id\":null}", "id"), None);
}
