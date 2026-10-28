//! Telling a tag from a comparison.

use rustx_compiler::scan::find_jsx;

#[test]
fn a_return_of_jsx_is_one_region() {
    let source = "fn App() -> Element { return <div>hi</div>; }";
    let spans = find_jsx(source).expect("valid");
    assert_eq!(spans.len(), 1);
    assert_eq!(&source[spans[0].start..spans[0].end], "<div>hi</div>");
}

#[test]
fn comparisons_are_not_mistaken_for_tags() {
    // The hardest part of the whole scanner: `<` means two things in Rust.
    let source = "fn f() { let ok = a < b && c > d; let g: Vec<u8> = vec![]; }";
    assert_eq!(find_jsx(source).expect("valid"), Vec::new());
}

#[test]
fn nested_tags_are_one_region_not_three() {
    let source = "fn App() -> Element { return <ul><li>a</li></ul>; }";
    let spans = find_jsx(source).expect("valid");
    assert_eq!(spans.len(), 1);
}

#[test]
fn an_unclosed_tag_names_the_rsx_line_and_column() {
    // Review Focus 1.
    let source = "fn App() -> Element {\n    return <div>oops;\n}";
    let error = find_jsx(source).expect_err("unclosed");
    assert_eq!(error.line, 2);
    assert_eq!(error.column, 12);
    assert!(error.message.contains("<div>"), "got {}", error.message);
}
