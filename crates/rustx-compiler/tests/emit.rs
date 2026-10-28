//! The Rust a JSX region becomes.

use rustx_compiler::emit::emit;
use rustx_compiler::parse::parse_jsx;
use rustx_compiler::{CompileError, scan::find_jsx};

fn node(source: &str) -> rustx_compiler::ast::Node {
    let span = find_jsx(source).expect("scannable")[0];
    parse_jsx(source, span).expect("parsable")
}

fn emit_one(source: &str) -> String {
    emit(&node(source)).expect("emittable")
}

fn emit_one_err(source: &str) -> CompileError {
    emit(&node(source)).expect_err("should refuse")
}

#[test]
fn a_tag_becomes_a_chain_of_constructors() {
    let rust = emit_one(r#"<div className="row">hi</div>"#);
    assert!(
        rust.contains(r#"Element :: tag ("div")"#) || rust.contains(r#"Element::tag("div")"#),
        "got {rust}"
    );
    assert!(rust.contains(r#"attribute ("class""#), "className should reach the DOM as class: {rust}");
    assert!(rust.contains("hi"), "got {rust}");
}

#[test]
fn an_event_attribute_becomes_a_handler_not_an_attribute() {
    let rust = emit_one("<button onClick={|_| ()}>go</button>");
    assert!(rust.contains("handler"), "got {rust}");
    assert!(!rust.contains("attribute (\"onClick\""), "got {rust}");
}

#[test]
fn an_unknown_attribute_is_refused_by_name() {
    // Review Focus 4: a typo must not be silently dropped.
    let error = emit_one_err(r#"<div classname="row" />"#);
    assert!(error.message.contains("classname"), "got {}", error.message);
    assert!(
        error.message.contains("className"),
        "a suggestion is expected: {}",
        error.message
    );
}

#[test]
fn text_is_emitted_as_a_string_literal_not_as_code() {
    // Review Focus 2, the other half: text must never become source.
    let rust = emit_one(r#"<p>{"}"} and " and \</p>"#);
    assert!(
        rust.contains("Element :: Text") || rust.contains("Element::Text"),
        "got {rust}"
    );
}

#[test]
fn an_expression_child_needs_no_conversion_from_the_developer() {
    // `{clicks}`, never `{clicks.to_string()}`.
    let rust = emit_one("<p>{count}</p>");
    assert!(rust.contains("into_element"), "got {rust}");
    assert!(
        !rust.contains("to_string"),
        "the developer's expression was rewritten: {rust}"
    );
}
