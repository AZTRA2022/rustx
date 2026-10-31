//! `<js::Chart />` — the tag that hands a node to React.

use rustx_compiler::transpile::transpile;

fn rewrite_one(source: &str) -> String {
    transpile(source).expect("transpilable").rust
}

fn refusal(source: &str) -> String {
    transpile(source).expect_err("should be refused").message
}

#[test]
fn a_js_tag_becomes_a_foreign_element() {
    let rust =
        rewrite_one("fn P() -> Element {\n    return <js::Chart series={\"a\".to_string()} />;\n}");
    assert!(rust.contains("Element :: Foreign"), "got {rust}");
    assert!(rust.contains("\"Chart\""), "got {rust}");
    assert!(rust.contains("\"series\""), "got {rust}");
}

#[test]
fn a_js_tag_gets_an_id_of_its_own() {
    let rust =
        rewrite_one("fn P() -> Element {\n    return <div><js::Chart /><js::Chart /></div>;\n}");
    let ids: Vec<&str> = rust.matches("rustx-js-").collect();
    assert_eq!(
        ids.len(),
        2,
        "each boundary needs its own container: {rust}"
    );
    // Deterministic, like a `Slot`: the line and the column it was
    // written at, so the engine finds the container without searching.
    assert!(rust.contains("rustx-js-2-17"), "got {rust}");
    assert!(rust.contains("rustx-js-2-30"), "got {rust}");
}

#[test]
fn a_prop_that_is_not_a_scalar_is_refused_by_name() {
    // Nothing is borrowed across the boundary: what crosses is null, a
    // boolean, a number or a string, copied in full.
    let message =
        refusal("fn P(camera: Camera) -> Element {\n    return <js::Chart camera={camera} />;\n}");
    assert!(message.contains("camera"), "got {message}");
}

#[test]
fn children_of_a_js_tag_are_refused() {
    // React owns what is inside; a child written here could never be
    // reconciled by either side.
    let message = refusal("fn P() -> Element {\n    return <js::Chart><p>hi</p></js::Chart>;\n}");
    assert!(message.contains("children"), "got {message}");
}

#[test]
fn a_lowercase_js_tag_is_still_refused_as_an_unknown_tag() {
    assert!(transpile("fn P() -> Element {\n    return <js::chart />;\n}").is_err());
}

#[test]
fn a_number_and_a_flag_cross_as_what_they_are() {
    let rust = rewrite_one("fn P() -> Element {\n    return <js::Chart height={240} live />;\n}");
    assert!(rust.contains("Number (240 as f64)"), "got {rust}");
    assert!(rust.contains("Bool (true)"), "got {rust}");
}

#[test]
fn a_literal_prop_crosses_as_a_string() {
    let rust = rewrite_one("fn P() -> Element {\n    return <js::Chart title=\"Live\" />;\n}");
    assert!(rust.contains("String (\"Live\""), "got {rust}");
}

#[test]
fn an_event_on_a_js_tag_is_refused() {
    // A handler is a Rust closure and cannot be given to React through
    // a container; the component takes a prop and calls it itself.
    let message = refusal("fn P() -> Element {\n    return <js::Chart onClick={|_| ()} />;\n}");
    assert!(message.contains("onClick"), "got {message}");
}

#[test]
fn a_number_a_page_computed_crosses_when_it_says_it_is_a_number() {
    // The compiler does not know the types of the developer's Rust, so
    // `as f64` is how a computed number says what it is — explicitly,
    // in the page, rather than by a guess in the compiler.
    let rust =
        rewrite_one("fn P() -> Element {\n    return <js::Chart points={count() as f64} />;\n}");
    assert!(rust.contains("Value :: Number"), "got {rust}");
    assert!(rust.contains("count () as f64"), "got {rust}");
}
