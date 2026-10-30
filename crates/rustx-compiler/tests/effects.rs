//! `keep`, `effect` and `task` get their call sites from the compiler.
//!
//! The developer writes `keep(Camera::open())`. Where that line sits is
//! what identifies the value across renders, and the compiler is the one
//! that knows it — so the compiler is the one that says it.

use rustx_compiler::transpile::transpile;

fn rewrite_one(source: &str) -> String {
    transpile(source).expect("transpilable").rust
}

#[test]
fn keep_gets_its_slot_from_where_it_is_written() {
    let rust = rewrite_one(
        "fn C() -> Element {\n    let c = keep(Camera::open());\n    return <p>{c.get()}</p>;\n}",
    );
    assert!(rust.contains("keep_at"), "got {rust}");
    assert!(rust.contains("line: 2u32"), "got {rust}");
}

#[test]
fn what_keep_is_given_becomes_what_builds_the_value() {
    // The argument is an expression the developer wrote; it has to reach
    // the cell unevaluated, or the camera would be opened every render.
    let rust = rewrite_one(
        "fn C() -> Element {\n    let c = keep(Camera::open());\n    return <p>{c.get()}</p>;\n}",
    );
    assert!(rust.contains("|| Camera::open()"), "got {rust}");
}

#[test]
fn effect_gets_its_slot_too() {
    let rust = rewrite_one(
        "fn C(id: u8) -> Element {\n    effect(id, || Camera::open(id));\n    return <p>ok</p>;\n}",
    );
    assert!(rust.contains("effect_at"), "got {rust}");
}

#[test]
fn effect_keeps_what_it_watches_and_what_it_runs_apart() {
    let rust = rewrite_one(
        "fn C(id: u8) -> Element {\n    effect(id, || Camera::open(id));\n    return <p>ok</p>;\n}",
    );
    assert!(
        rust.contains("}, id, || Camera::open(id))"),
        "the two arguments were not kept apart: {rust}"
    );
}

#[test]
fn a_task_is_a_value_the_component_keeps() {
    // This is all that ties a thread to a component's life: the token is
    // a kept value, so it is dropped at unmount, so the task stops.
    let rust = rewrite_one(
        "fn C() -> Element {\n    task(move |alive| while alive.get() {});\n    return <p>ok</p>;\n}",
    );
    assert!(rust.contains("keep_at"), "got {rust}");
    assert!(
        rust.contains("rustx_engine::task::spawn(move |alive| while alive.get() {})"),
        "got {rust}"
    );
}

#[test]
fn a_keep_outside_a_component_is_left_alone() {
    // `keep` is an ordinary name outside a `fn … -> Element`.
    let source = "fn helper() -> u8 { let keep = 1; keep }";
    assert_eq!(rewrite_one(source), source);
}

#[test]
fn a_name_that_is_not_a_call_is_left_alone() {
    // A component may call a local `keep`. Only a call is rewritten; the
    // JSX around it is transpiled as it always is.
    let source = "fn C() -> Element {\n    let keep = 1;\n    return <p>{keep}</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(!rust.contains("keep_at"), "got {rust}");
    assert!(rust.contains("let keep = 1;"), "got {rust}");
}

#[test]
fn the_rewrite_still_keeps_the_file_the_same_length() {
    let source =
        "fn C() -> Element {\n    let c = keep(Camera::open());\n    return <p>{c.get()}</p>;\n}\n";
    assert_eq!(
        transpile(source).unwrap().rust.lines().count(),
        source.lines().count()
    );
}
