//! `<js::recharts::LineChart />` — the tag that is a component off npm.

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
fn a_library_component_is_named_by_its_module() {
    // What the developer writes is the import they would have written in
    // JavaScript: the module, then the export.
    let rust =
        rewrite_one("fn P() -> Element {\n    return <js::recharts::LineChart width={320} />;\n}");
    assert!(rust.contains("\"recharts::LineChart\""), "got {rust}");
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
fn a_prop_crosses_through_to_value() {
    // The compiler does not know the types of the developer's Rust, and
    // it does not have to: `ToValue` is what says a type can cross, and
    // a type that cannot is refused by `rustc` at the line in the `.rsx`
    // that wrote it — which is more than a guess in the compiler could
    // ever say.
    let rust = rewrite_one(
        "fn P(points: Vec<Point>) -> Element {\n    \
         return <js::recharts::LineChart data={points.clone()} />;\n}",
    );
    assert!(rust.contains("ToValue :: to_value"), "got {rust}");
    assert!(rust.contains("points . clone ()"), "got {rust}");
}

#[test]
fn a_foreign_tag_may_hold_foreign_children() {
    // A chart is written with its axes inside it, the way it is written
    // in JSX — and the whole tree is handed over at once.
    let rust = rewrite_one(
        "fn P() -> Element {\n    return <js::recharts::LineChart width={320}>\n        \
         <js::recharts::XAxis dataKey=\"at\" />\n    </js::recharts::LineChart>;\n}",
    );
    assert!(rust.contains("\"recharts::XAxis\""), "got {rust}");
    assert!(rust.contains("ForeignChild :: Node"), "got {rust}");
}

#[test]
fn text_inside_a_foreign_tag_is_a_child_of_it() {
    let rust = rewrite_one("fn P() -> Element {\n    return <js::Button>Save</js::Button>;\n}");
    assert!(rust.contains("ForeignChild :: Text"), "got {rust}");
}

#[test]
fn a_lowercase_js_tag_is_still_refused_as_an_unknown_tag() {
    assert!(transpile("fn P() -> Element {\n    return <js::chart />;\n}").is_err());
}

#[test]
fn a_lowercase_export_in_a_module_is_refused_by_name() {
    let message = refusal("fn P() -> Element {\n    return <js::recharts::lineChart />;\n}");
    assert!(message.contains("LineChart"), "got {message}");
}

#[test]
fn a_number_and_a_flag_cross_as_what_they_are() {
    let rust = rewrite_one("fn P() -> Element {\n    return <js::Chart height={240} live />;\n}");
    assert!(rust.contains("ToValue :: to_value"), "got {rust}");
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
fn a_capitalised_tag_the_file_does_not_declare_is_a_library_component() {
    // No prefix. A tag the page declares is the page's; a tag it does
    // not is a component off npm, found by its name among the libraries
    // the application installed.
    let rust = rewrite_one("fn P() -> Element {\n    return <LineChart width={320} />;\n}");
    assert!(rust.contains("Element :: Foreign"), "got {rust}");
    assert!(rust.contains("\"LineChart\""), "got {rust}");
}

#[test]
fn a_dotted_tag_is_a_path_into_a_library() {
    // `<Dialog.Trigger />`, as it is written in JSX.
    let rust =
        rewrite_one("fn P() -> Element {\n    return <Dialog.Trigger>open</Dialog.Trigger>;\n}");
    assert!(rust.contains("\"Dialog.Trigger\""), "got {rust}");
    assert!(rust.contains("ForeignChild :: Text"), "got {rust}");
}

#[test]
fn a_tag_the_file_declares_is_still_a_rustx_component() {
    let rust = rewrite_one(
        "fn P() -> Element {\n    return <Row name={\"a\".to_string()} />;\n}\n\
         fn Row(name: String) -> Element {\n    return <li>{name}</li>;\n}",
    );
    assert!(!rust.contains("Element :: Foreign"), "got {rust}");
    assert!(rust.contains("component_at"), "got {rust}");
}

#[test]
fn a_plain_tag_inside_a_library_tag_is_handed_over_as_itself() {
    // React builds what is inside the boundary, so a `<div>` there is
    // React's `createElement("div", …)` — not a node rustx thinks it
    // owns. It is how anyone wraps the inside of a dialog.
    let rust = rewrite_one(
        "fn P() -> Element {\n    return <Content><div className=\"flex\">hi</div></Content>;\n}",
    );
    assert!(rust.contains("\"Content\""), "got {rust}");
    assert!(rust.contains("\"div\""), "got {rust}");
    assert!(rust.contains("ForeignChild :: Node"), "got {rust}");
}

#[test]
fn an_unknown_tag_inside_a_library_tag_is_still_refused() {
    let message = refusal("fn P() -> Element {\n    return <Content><dvi>hi</dvi></Content>;\n}");
    assert!(message.contains("dvi"), "got {message}");
}
