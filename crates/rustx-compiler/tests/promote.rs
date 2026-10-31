//! Which locals become state, and which stay plain Rust.

use rustx_compiler::promote::{MutationKind, analyse};

/// Exactly the counter of the design document, §7.4.
const COUNTER: &str = "fn Counter() -> Element {\n    let mut n = 0;\n    return <button onClick={|| n += 1}>{n}</button>;\n}\n";

#[test]
fn a_variable_read_in_the_jsx_and_mutated_in_a_handler_is_promoted() {
    let components = analyse(COUNTER).unwrap();
    let promoted: Vec<&str> = components[0]
        .promoted
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(promoted, ["n"]);
}

#[test]
fn a_variable_only_mutated_in_a_handler_is_not_promoted() {
    // Review Focus 2. Nothing reads it, so nothing needs to survive.
    let source = "fn C() -> Element {\n    let mut log = 0;\n    return <button onClick={|| log += 1}>go</button>;\n}";
    assert!(analyse(source).unwrap()[0].promoted.is_empty());
}

#[test]
fn a_variable_only_read_in_the_jsx_is_not_promoted() {
    // The other half: an ordinary local, recomputed each render, pays
    // nothing.
    let source = "fn C() -> Element {\n    let title = \"hi\";\n    return <h1>{title}</h1>;\n}";
    assert!(analyse(source).unwrap()[0].promoted.is_empty());
}

#[test]
fn a_mutation_outside_a_handler_does_not_promote() {
    let source = "fn C() -> Element {\n    let mut n = 0;\n    n += 1;\n    return <p>{n}</p>;\n}";
    assert!(analyse(source).unwrap()[0].promoted.is_empty());
}

#[test]
fn a_component_parameter_is_never_promoted() {
    let source =
        "fn C(label: String) -> Element {\n    return <button onClick={|| ()}>{label}</button>;\n}";
    assert!(analyse(source).unwrap()[0].promoted.is_empty());
}

#[test]
fn each_kind_of_mutation_is_recognised() {
    let source = "fn C() -> Element {\n    let mut items = vec![];\n    let mut n = 0;\n    let mut s = 0;\n    return <div onClick={|| { items.push(1); n = 5; s += 1; }}>{items.len()}{n}{s}</div>;\n}";
    let kinds: Vec<MutationKind> = analyse(source).unwrap()[0]
        .promoted
        .iter()
        .flat_map(|p| p.mutations.iter().map(|m| m.kind))
        .collect();
    assert!(kinds.contains(&MutationKind::Method), "got {kinds:?}");
    assert!(kinds.contains(&MutationKind::Assign), "got {kinds:?}");
    assert!(kinds.contains(&MutationKind::Compound), "got {kinds:?}");
}

#[test]
fn a_function_that_does_not_return_element_is_not_a_component() {
    let source = "fn helper() -> u8 { let mut n = 0; n += 1; n }";
    assert!(analyse(source).unwrap().is_empty());
}

#[test]
fn the_declaration_and_its_initializer_are_located() {
    let promoted = &analyse(COUNTER).unwrap()[0].promoted[0];
    assert_eq!(
        &COUNTER[promoted.initializer.start..promoted.initializer.end],
        "0"
    );
    assert!(COUNTER[promoted.declaration.start..promoted.declaration.end].starts_with("let mut n"));
}
