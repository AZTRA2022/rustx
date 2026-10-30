//! Turning a JSX region into a node tree.

use rustx_compiler::ast::{AttributeSource, Node};
use rustx_compiler::parse::parse_jsx;
use rustx_compiler::{CompileError, scan::find_jsx};

fn parse_one(source: &str) -> Node {
    let span = find_jsx(source).expect("scannable")[0];
    parse_jsx(source, span).expect("parsable")
}

fn parse_one_err(source: &str) -> CompileError {
    match find_jsx(source) {
        Err(error) => error,
        Ok(spans) => parse_jsx(source, spans[0]).expect_err("should refuse"),
    }
}

#[test]
fn text_children_keep_their_punctuation() {
    // Review Focus 2: `<`, `&` and quotes in UI text are ordinary text.
    let node = parse_one(r#"<p>10 < 20 & "quoted"</p>"#);
    let Node::Tag(tag) = node else { panic!() };
    assert_eq!(tag.children.len(), 1);
    assert!(matches!(&tag.children[0], Node::Text(t) if t == r#"10 < 20 & "quoted""#));
}

#[test]
fn an_expression_child_keeps_its_rust_verbatim() {
    let node = parse_one("<p>{ user.name.to_uppercase() }</p>");
    let Node::Tag(tag) = node else { panic!() };
    assert!(matches!(&tag.children[0], Node::Expression { code, .. } if code.trim() == "user.name.to_uppercase()"));
}

#[test]
fn attributes_come_in_three_shapes() {
    let node = parse_one(r#"<input className="row" value={name} disabled />"#);
    let Node::Tag(tag) = node else { panic!() };
    assert!(matches!(&tag.attributes[0].value, AttributeSource::Literal(v) if v == "row"));
    assert!(matches!(&tag.attributes[1].value, AttributeSource::Expression(v) if v.trim() == "name"));
    assert!(matches!(tag.attributes[2].value, AttributeSource::Flag));
}

#[test]
fn a_closure_attribute_survives_its_braces_and_body() {
    let node = parse_one("<button onClick={|| { let mut n = 0; n += 1; }}>go</button>");
    let Node::Tag(tag) = node else { panic!() };
    let AttributeSource::Expression(code) = &tag.attributes[0].value else {
        panic!()
    };
    assert!(code.contains("n += 1"), "got {code}");
}

#[test]
fn a_self_closing_tag_has_no_children() {
    let node = parse_one("<br />");
    let Node::Tag(tag) = node else { panic!() };
    assert!(tag.children.is_empty());
}

#[test]
fn a_mismatched_closing_tag_says_which_two_disagree() {
    let error = parse_one_err("<div>hi</span>");
    assert!(error.message.contains("div"), "got {}", error.message);
    assert!(error.message.contains("span"), "got {}", error.message);
}

#[test]
fn a_space_between_text_and_an_inline_tag_survives() {
    // `a <b>bold</b> word` is the most ordinary markup there is, and
    // trimming every text node renders it "aboldword".
    let node = parse_one("<p>Hello <b>world</b> again</p>");
    let Node::Tag(tag) = node else { panic!() };
    assert!(
        matches!(&tag.children[0], Node::Text(t) if t == "Hello "),
        "got {:?}",
        tag.children[0]
    );
    assert!(
        matches!(&tag.children[2], Node::Text(t) if t == " again"),
        "got {:?}",
        tag.children[2]
    );
}

#[test]
fn indentation_does_not_leak_into_the_rendered_text() {
    // Whitespace that crosses a newline is layout, not content: JSX drops
    // it, and a tree written over several indented lines must not print
    // its own indentation.
    let node = parse_one("<p>\n    line one\n    line two\n</p>");
    let Node::Tag(tag) = node else { panic!() };
    assert_eq!(tag.children.len(), 1);
    assert!(
        matches!(&tag.children[0], Node::Text(t) if t == "line one line two"),
        "got {:?}",
        tag.children[0]
    );
}

#[test]
fn whitespace_between_tags_on_separate_lines_is_dropped() {
    let node = parse_one("<ul>\n    <li>a</li>\n    <li>b</li>\n</ul>");
    let Node::Tag(tag) = node else { panic!() };
    assert_eq!(tag.children.len(), 2, "got {:?}", tag.children);
}

#[test]
fn an_attribute_string_has_no_escapes_just_like_in_jsx() {
    // A Windows path in a tooltip. The scanner reads Rust escapes, but
    // an attribute's quotes are HTML's, not Rust's: a backslash is a
    // backslash, and the value must not grow one.
    let node = parse_one(r#"<div title="C:\"></div>"#);
    let Node::Tag(tag) = node else { panic!() };
    assert!(
        matches!(&tag.attributes[0].value, AttributeSource::Literal(v) if v == "C:\\"),
        "got {:?}",
        tag.attributes[0].value
    );
}
