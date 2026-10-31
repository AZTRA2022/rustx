//! The Rust a JSX region becomes.

use rustx_compiler::emit::{Context, emit};
use rustx_compiler::parse::parse_jsx;
use rustx_compiler::{CompileError, scan::find_jsx};

fn node(source: &str) -> rustx_compiler::ast::Node {
    let span = find_jsx(source).expect("scannable")[0];
    parse_jsx(source, span).expect("parsable")
}

/// A file with no components of its own, which is what these tests are
/// about: the elements.
fn context(source: &str) -> Context<'_> {
    Context {
        components: &[],
        source,
    }
}

fn emit_one(source: &str) -> String {
    emit(&node(source), &context(source)).expect("emittable")
}

fn emit_one_err(source: &str) -> CompileError {
    emit(&node(source), &context(source)).expect_err("should refuse")
}

#[test]
fn a_tag_becomes_a_chain_of_constructors() {
    let rust = emit_one(r#"<div className="row">hi</div>"#);
    assert!(
        rust.contains(r#"Element :: tag ("div")"#) || rust.contains(r#"Element::tag("div")"#),
        "got {rust}"
    );
    assert!(
        rust.contains(r#"attribute ("class""#),
        "className should reach the DOM as class: {rust}"
    );
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

#[test]
fn an_unknown_tag_is_refused_by_name() {
    // Props fail closed; tags must too. `<dvi>` becomes an inert
    // HTMLUnknownElement and a blank area on screen — silence is exactly
    // what the document forbids.
    let error = emit_one_err(r#"<dvi className="x"></dvi>"#);
    assert!(error.message.contains("dvi"), "got {}", error.message);
    assert!(
        error.message.contains("div"),
        "no suggestion: {}",
        error.message
    );
}

#[test]
fn a_component_tag_says_components_are_not_here_yet() {
    // Every TSX author will try `<Card />` on the first day. Telling them
    // it is an unknown HTML tag would send them looking for a typo.
    let error = emit_one_err("<Card />");
    assert!(
        error.message.contains("component"),
        "an unhelpful message for a PascalCase tag: {}",
        error.message
    );
}

#[test]
fn an_icon_can_be_drawn() {
    // No SVG attributes meant no icon could be drawn at all.
    let rust = emit_one(
        r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg"><path d="M1 1 L8 8" stroke="currentColor" fill="none" /></svg>"#,
    );
    assert!(rust.contains("viewBox"), "got {rust}");
    assert!(rust.contains("stroke"), "got {rust}");
}

#[test]
fn a_suggestion_is_only_offered_when_it_is_plausible() {
    // `bar` once suggested `dir`, which helps nobody.
    let error = emit_one_err(r#"<div bar="1"></div>"#);
    assert!(
        !error.message.contains("did you mean"),
        "noise instead of silence: {}",
        error.message
    );
}
