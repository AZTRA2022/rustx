//! `style={{ … }}`, the way TSX writes it.

use rustx_compiler::attributes::style_string;
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
