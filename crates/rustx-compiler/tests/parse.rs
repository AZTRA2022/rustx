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
    assert!(matches!(&tag.children[0], Node::Expression(e) if e.trim() == "user.name.to_uppercase()"));
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
