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

#[test]
fn a_call_written_inside_another_is_refused_rather_than_mangled() {
    // Two edits over the same bytes shred each other: the inner one is
    // applied first and the outer one then replaces a range whose end
    // has moved. `keep(task(…))` is the first thing a reader of the
    // architecture would write, so it gets a sentence.
    let error = transpile(
        "fn C() -> Element {\n    let t = keep(task(move |alive| while alive.get() {}));\n    return <p>ok</p>;\n}",
    )
    .expect_err("a nested call should be refused");
    assert_eq!(error.line, 2, "got {error:?}");
    assert!(
        error.message.contains("keep") && error.message.contains("task"),
        "the complaint should name both calls: {}",
        error.message
    );
}

#[test]
fn a_qualified_call_is_somebody_elses() {
    // `Self::keep(…)` and `camera::keep(…)` are not ours.
    let source = "fn C() -> Element {\n    let c = Self::keep(1);\n    return <p>{c}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(!rust.contains("keep_at"), "got {rust}");
    assert!(rust.contains("Self::keep(1)"), "got {rust}");
}

#[test]
fn a_call_whose_arguments_span_several_lines_keeps_the_file_the_same_length() {
    // What rustfmt does to a long argument. Plan 2's whole position
    // story rests on the line count, and after one such call every
    // rustc line number in the rest of the file would be wrong.
    let source = "fn C() -> Element {\n    let c = keep(\n        Camera::open(),\n    );\n    return <p>{c.get()}</p>;\n}\n";
    assert_eq!(
        transpile(source).unwrap().rust.lines().count(),
        source.lines().count(),
        "got\n{}",
        transpile(source).unwrap().rust
    );
}

#[test]
fn an_effect_whose_arguments_span_several_lines_keeps_the_file_the_same_length() {
    let source = "fn C(id: u8) -> Element {\n    effect(\n        id,\n        || Camera::open(id),\n    );\n    return <p>ok</p>;\n}\n";
    assert_eq!(
        transpile(source).unwrap().rust.lines().count(),
        source.lines().count(),
        "got\n{}",
        transpile(source).unwrap().rust
    );
}

#[test]
fn a_local_named_keep_is_refused_rather_than_hijacked() {
    // The three names belong to the framework inside a component. A
    // component that declares one of them as a local is told, instead
    // of having its own call rewritten into something else.
    let error = transpile(
        "fn C() -> Element {\n    let keep = |x: u8| x;\n    return <p>{keep(1)}</p>;\n}",
    )
    .expect_err("a local named keep should be refused");
    assert!(error.message.contains("keep"), "got {}", error.message);
}

#[test]
fn a_call_inside_a_nested_fn_is_left_alone() {
    // A nested `fn` is not a component: its `keep` has no instance to
    // belong to, and attaching it to whatever happens to be rendering
    // is worse than leaving it as the error it is.
    let source = "fn C() -> Element {\n    fn helper() -> u8 { keep(1) }\n    return <p>{helper()}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(!rust.contains("keep_at"), "got {rust}");
}
