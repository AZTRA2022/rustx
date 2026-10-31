//! What a tag may be given: the props of HTML, and the props of JSX.
//!
//! A `.rsx` is written by someone who knows one of the two dialects and
//! often both. `class` and `className` are the same attribute; so are
//! `tabindex` and `tabIndex`, `stroke-width` and `strokeWidth`. Refusing
//! either spelling is refusing a page nobody would think was wrong.

use rustx_compiler::attributes::{dom_name, is_known};
use rustx_compiler::emit::{Context, emit};
use rustx_compiler::parse::parse_jsx;
use rustx_compiler::scan::find_jsx;

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

fn emit_err(source: &str) -> String {
    let span = find_jsx(source).expect("scannable")[0];
    let node = parse_jsx(source, span).expect("parsable");
    emit(
        &node,
        &Context {
            components: &[],
            source,
        },
    )
    .expect_err("should refuse")
    .message
}

#[test]
fn both_spellings_of_a_class_reach_the_dom_as_class() {
    assert_eq!(dom_name("className"), "class");
    assert_eq!(dom_name("class"), "class");
    assert!(emit_one("<p class=\"a\">x</p>").contains("\"class\""));
    assert!(emit_one("<p className=\"a\">x</p>").contains("\"class\""));
}

#[test]
fn both_spellings_of_a_label_target_reach_the_dom_as_for() {
    assert_eq!(dom_name("htmlFor"), "for");
    assert_eq!(dom_name("for"), "for");
}

#[test]
fn an_html_spelling_is_known_wherever_the_jsx_one_is() {
    for name in [
        "tabindex",
        "maxlength",
        "minlength",
        "readonly",
        "colspan",
        "rowspan",
        "autocomplete",
        "autofocus",
        "crossorigin",
        "srcset",
        "contenteditable",
        "spellcheck",
        "inputmode",
        "enterkeyhint",
        "playsinline",
        "referrerpolicy",
        "accesskey",
        "novalidate",
        "formaction",
        "datetime",
        "allowfullscreen",
        "itemprop",
    ] {
        assert!(is_known(name), "`{name}` should be a known attribute");
    }
}

#[test]
fn an_html_attribute_keeps_its_own_spelling_in_the_dom() {
    assert_eq!(dom_name("tabindex"), "tabindex");
    assert_eq!(dom_name("tabIndex"), "tabindex");
    assert_eq!(dom_name("maxlength"), "maxlength");
    assert_eq!(dom_name("maxLength"), "maxlength");
}

#[test]
fn the_attributes_a_page_of_html_actually_uses_are_known() {
    for name in [
        "charset",
        "content",
        "http-equiv",
        "media",
        "sizes",
        "integrity",
        "defer",
        "async",
        "nonce",
        "sandbox",
        "srcdoc",
        "loop",
        "kind",
        "srclang",
        "default",
        "label",
        "span",
        "coords",
        "shape",
        "ping",
        "hreflang",
        "usemap",
        "ismap",
        "high",
        "low",
        "optimum",
        "slot",
        "inert",
        "popover",
        "capture",
        "autocapitalize",
        "is",
    ] {
        assert!(is_known(name), "`{name}` should be a known attribute");
    }
}

#[test]
fn the_presentation_attributes_an_icon_needs_are_known_in_both_spellings() {
    for (jsx, html) in [
        ("fontSize", "font-size"),
        ("fontFamily", "font-family"),
        ("fontWeight", "font-weight"),
        ("letterSpacing", "letter-spacing"),
        ("dominantBaseline", "dominant-baseline"),
        ("strokeMiterlimit", "stroke-miterlimit"),
        ("pointerEvents", "pointer-events"),
        ("markerEnd", "marker-end"),
        ("floodColor", "flood-color"),
        ("paintOrder", "paint-order"),
    ] {
        assert!(is_known(jsx), "`{jsx}` should be known");
        assert!(is_known(html), "`{html}` should be known");
        assert_eq!(dom_name(jsx), html, "`{jsx}` should reach the DOM as it");
        assert_eq!(dom_name(html), html);
    }
}

#[test]
fn the_open_namespaces_stay_open() {
    assert!(is_known("data-device"));
    assert!(is_known("aria-label"));
    assert!(is_known("xlink:href"));
    assert!(is_known("xml:lang"));
}

#[test]
fn a_typo_is_still_refused_and_still_suggests() {
    // Accepting both spellings is not accepting anything: a dropped
    // `classname` is an afternoon spent wondering why the style does
    // not apply.
    let message = emit_err("<p classname=\"a\">x</p>");
    assert!(message.contains("className"), "got {message}");
    assert!(!is_known("clas"));
    assert!(!is_known("onclick"), "an event is React's spelling only");
}

#[test]
fn an_attribute_written_in_html_still_takes_a_value_the_same_way() {
    let rust = emit_one("<input readonly disabled maxlength=\"4\" />");
    assert!(rust.contains("\"readonly\""), "got {rust}");
    assert!(rust.contains("\"maxlength\""), "got {rust}");
    assert!(rust.contains("Flag (true)"), "got {rust}");
}

#[test]
fn a_key_identifies_a_row_and_the_state_inside_it() {
    // §7.4 bis asks for `key`, with React's meaning: it identifies the
    // row for the reconciler *and* the state inside it, so a reorder
    // carries each row's own count with it. One attribute, both.
    let rust = emit_one("<li key={name.clone()}>x</li>");
    assert!(rust.contains(". key ("), "got {rust}");
    assert!(rust.contains("with_key"), "got {rust}");
    assert!(
        !rust.contains("\"key\""),
        "`key` is not a DOM attribute: {rust}"
    );
}
