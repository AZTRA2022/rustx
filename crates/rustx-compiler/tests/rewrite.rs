//! What a promoted local turns into.

use rustx_compiler::transpile::transpile;

const COUNTER: &str = "fn Counter() -> Element {\n    let mut n = 0;\n    return <button onClick={|| n += 1}>{n}</button>;\n}\n";

fn rewrite_one(source: &str) -> String {
    transpile(source).expect("transpilable").rust
}

#[test]
fn the_declaration_becomes_a_cell_identified_by_its_place_in_the_code() {
    let rust = rewrite_one(COUNTER);
    assert!(rust.contains("rustx_dom::state::cell"), "got {rust}");
    assert!(rust.contains("Slot"), "got {rust}");
    assert!(!rust.contains("let mut n = 0"), "the plain local survived: {rust}");
}

#[test]
fn a_compound_mutation_in_a_handler_becomes_an_update() {
    let rust = rewrite_one(COUNTER);
    assert!(rust.contains("update"), "got {rust}");
    assert!(!rust.contains("n += 1"), "the raw mutation survived: {rust}");
}

#[test]
fn a_handler_closure_owns_its_handles() {
    // The handler outlives the render that built it, so it cannot borrow.
    let rust = rewrite_one(COUNTER);
    assert!(rust.contains("move |"), "got {rust}");
    assert!(rust.contains("clone"), "got {rust}");
}

#[test]
fn a_read_in_the_jsx_becomes_a_read_of_the_cell() {
    // The developer writes `{n}` and nothing else; the conversion is the
    // compiler's. It is a read of the cell, not a copy of the handle:
    // `into_element` takes its argument by value, so handing it the
    // handle would move it.
    let rust = rewrite_one(COUNTER);
    assert!(rust.contains("n . get ()"), "got {rust}");
}

#[test]
fn the_same_cell_can_be_read_twice_in_one_tree() {
    let source = "fn C() -> Element {\n    let mut n = 0;\n    return <div onClick={|| n += 1}>{n}{n}</div>;\n}";
    let rust = rewrite_one(source);
    assert_eq!(rust.matches("n . get ()").count(), 2, "got {rust}");
}

#[test]
fn a_plain_assignment_becomes_a_set() {
    let source = "fn C() -> Element {\n    let mut n = 0;\n    return <button onClick={|| n = 5}>{n}</button>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("set"), "got {rust}");
}

#[test]
fn a_method_mutation_goes_through_update_not_through_a_copy() {
    let source = "fn C() -> Element {\n    let mut items: Vec<u8> = Vec::new();\n    return <button onClick={|| { items.push(1); }}>{items.len()}</button>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("update"), "got {rust}");
    assert!(!rust.contains("get () . push"), "the push went to a copy: {rust}");
}

#[test]
fn an_untouched_component_is_left_exactly_as_it_was() {
    let source = "fn C() -> Element {\n    let title = \"hi\";\n    return <h1>{title}</h1>;\n}\n";
    let rust = rewrite_one(source);
    assert!(!rust.contains("state::cell"), "got {rust}");
    assert!(rust.contains("let title = \"hi\""), "got {rust}");
}

#[test]
fn the_rewrite_keeps_the_file_the_same_length() {
    // The line map of plan 2 still has to hold: a rewrite that moved
    // lines would put every rustc error back where it was.
    let out = transpile(COUNTER).unwrap();
    assert_eq!(out.rust.lines().count(), COUNTER.lines().count(), "{}", out.rust);
}
