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

/// Rust whose `<` sits exactly where a tag's would, and is not one.
///
/// The scanner's rule looks at the token before and the byte after. A
/// qualified path satisfies both halves, and it is how anyone
/// disambiguates a trait method — not exotic Rust at all.
#[test]
fn a_qualified_path_is_not_a_tag() {
    for source in [
        "fn f() { let x = <T as Default>::default(); }",
        "fn f() { g(1, <T as Default>::default()); }",
        "fn f() { return <T as Default>::default(); }",
        "fn f() { <T as Tr>::go(); }",
    ] {
        assert_eq!(
            find_jsx(source).expect(source),
            Vec::new(),
            "mis-read as a tag: {source}"
        );
    }
}

#[test]
fn a_raw_string_is_not_scanned_for_escapes_or_quotes() {
    // A Windows path literal, on a Windows-only framework.
    for source in [
        r#"fn f() { let p = r"C:\temp\"; let q = 1; }"#,
        r##"fn f() { let p = r#"a " b"#; let q = 2; }"##,
        r#"fn f() { let p = b"bytes\\"; let q = 3; }"#,
    ] {
        assert_eq!(find_jsx(source).expect(source), Vec::new(), "{source}");
    }
}

#[test]
fn a_raw_string_inside_jsx_does_not_end_the_region_early() {
    let source = r##"fn App() -> Element { return <p>{ r#"a " b"# }</p>; }"##;
    let spans = find_jsx(source).expect("valid");
    assert_eq!(spans.len(), 1);
    assert!(
        source[spans[0].start..spans[0].end].ends_with("</p>"),
        "got {:?}",
        &source[spans[0].start..spans[0].end]
    );
}
