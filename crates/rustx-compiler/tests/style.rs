//! `style={{ … }}`, the way TSX writes it.

use rustx_compiler::attributes::{Styles, style_string, style_value};
use rustx_compiler::emit::{Context, emit};
use rustx_compiler::parse::parse_jsx;
use rustx_compiler::scan::find_jsx;
use rustx_compiler::transpile::transpile;

fn emit_one(source: &str) -> String {
    let span = find_jsx(source).expect("scannable")[0];
    let node = parse_jsx(source, span).expect("parsable");
    emit(
        &node,
        &Context {
            components: &[],
            source,
        },
    )
    .expect("emittable")
}

#[test]
fn an_object_of_styles_becomes_one_css_declaration() {
    assert_eq!(
        style_string("{ display: \"flex\", gap: \"12px\" }").unwrap(),
        "display: flex; gap: 12px"
    );
}

#[test]
fn a_camel_case_property_becomes_kebab_case() {
    // Review Focus 2, second half: an already-kebab key is left alone.
    assert_eq!(
        style_string("{ backgroundColor: \"#09090b\" }").unwrap(),
        "background-color: #09090b"
    );
    assert_eq!(
        style_string("{ \"background-color\": \"#09090b\" }").unwrap(),
        "background-color: #09090b"
    );
}

#[test]
fn a_trailing_comma_is_allowed_because_rustfmt_writes_one() {
    assert_eq!(style_string("{ color: \"red\", }").unwrap(), "color: red");
}

#[test]
fn a_value_with_a_comma_in_it_survives() {
    // `rgba(0,0,0,.5)` and font stacks are the common case.
    assert_eq!(
        style_string("{ color: \"rgba(255,255,255,0.1)\", fontFamily: \"a, b\" }").unwrap(),
        "color: rgba(255,255,255,0.1); font-family: a, b"
    );
}

#[test]
fn a_numeric_value_is_refused_by_name() {
    // Review Focus 2: React adds `px` to some numbers and not others.
    // Guessing is worse than asking.
    let complaint = style_string("{ width: 10 }").expect_err("should be refused");
    assert!(
        complaint.contains("width") && complaint.contains("px"),
        "got {complaint}"
    );
}

#[test]
fn an_empty_object_is_an_empty_declaration() {
    assert_eq!(style_string("{}").unwrap(), "");
}

#[test]
fn a_style_object_reaches_the_dom_as_a_style_attribute() {
    let rust = emit_one("<div style={{ display: \"flex\" }}>hi</div>");
    assert!(rust.contains("\"style\""), "got {rust}");
    assert!(rust.contains("display: flex"), "got {rust}");
}

#[test]
fn a_style_string_is_still_a_style_string() {
    let rust = emit_one("<div style=\"display: flex\">hi</div>");
    assert!(rust.contains("display: flex"), "got {rust}");
}

#[test]
fn a_style_expression_that_is_not_an_object_is_passed_through() {
    // `style={computed}` is a value the developer built; it reaches the
    // DOM as it is.
    let rust = emit_one("<div style={computed}>hi</div>");
    assert!(rust.contains("computed"), "got {rust}");
}

#[test]
fn a_style_object_inside_a_component_compiles() {
    // The whole point: a page written the way TSX is written.
    let source =
        "fn P() -> Element {\n    return <div style={{ padding: \"20px 32px\" }}>hi</div>;\n}\n";
    let rust = transpile(source).expect("transpilable").rust;
    assert!(rust.contains("padding: 20px 32px"), "got {rust}");
    assert_eq!(rust.lines().count(), source.lines().count());
}

#[test]
fn a_style_built_with_format_is_passed_through() {
    // The escape hatch for a computed style, and the one that is
    // actually reached for. The braces of `{width}` are inside a string
    // literal, so the scanner must not take them for the opening of a
    // style object -- which is the test that distinguishes this from
    // `style={computed}` above.
    let source = "fn P() -> Element {\n    \
         return <div style={format!(\"width: {width}px\")}>hi</div>;\n}\n";
    let rust = transpile(source).expect("transpilable").rust;
    assert!(
        rust.contains("format !") || rust.contains("format!"),
        "got {rust}"
    );
    assert!(rust.contains("width"), "got {rust}");
    assert_eq!(rust.lines().count(), source.lines().count());
}

#[test]
fn a_style_object_may_hold_a_computed_value() {
    // This used to be refused, and the advice given was to write
    // `"format!(…)px"` -- which cannot be followed, because the object
    // is read at compile time and a string is a string. The object is
    // read into the `format!` the developer would have written.
    let source = "fn P() -> Element {\n    \
         return <div style={{ width: format!(\"{w}px\") }}>hi</div>;\n}\n";
    let rust = transpile(source).expect("a computed style compiles").rust;
    // A pretty-printed token stream writes `format !`, so the macro is
    // looked for by name rather than by its punctuation.
    assert!(rust.contains("format"), "got {rust}");
    assert!(rust.contains("width: {}"), "got {rust}");
    assert!(rust.contains("{w}px"), "got {rust}");
}

#[test]
fn a_computed_value_beside_a_literal_one_keeps_both() {
    let made = style_value("{ width: wide, color: \"red\" }").expect("a style");
    assert_eq!(
        made,
        Styles::Computed("format!(\"width: {}; color: red\", wide)".to_string())
    );
}

/// A `format!` reads `{` as the start of a placeholder, so a literal
/// that carries one has to be doubled -- and a declaration that never
/// becomes a pattern must not be.
#[test]
fn a_brace_inside_a_literal_value_survives_either_way() {
    let plain = style_value("{ background: \"image-set(a {1x})\" }").expect("a style");
    assert_eq!(
        plain,
        Styles::Text("background: image-set(a {1x})".to_string())
    );

    let mixed = style_value("{ background: \"image-set(a {1x})\", width: wide }").expect("a style");
    let Styles::Computed(source) = mixed else {
        panic!("a computed style was expected, got {mixed:?}");
    };
    assert!(
        source.contains("{{1x}}"),
        "the brace was not doubled: {source}"
    );
}

/// A number is still refused, and for the original reason: React adds
/// `px` to some properties and not to others.
#[test]
fn a_numeric_value_is_still_refused_now_that_expressions_are_allowed() {
    let complaint = style_value("{ width: 10 }").expect_err("should be refused");
    assert!(
        complaint.contains("width") && complaint.contains("px"),
        "got {complaint}"
    );
}
