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
fn a_variable_only_mutated_in_a_handler_is_promoted_anyway() {
    // Review Focus 2 used to say the opposite: nothing reads it, so
    // nothing needs to survive. What the opposite produced was Rust
    // that does not compile. A handler is stored as an `Rc<dyn Fn>`,
    // and a closure that assigns to a captured local cannot be one,
    // so the developer got a borrow-check error about a generated
    // file they had never opened. One cell is cheaper than that.
    let source = "fn C() -> Element {\n    let mut log = 0;\n    return <button onClick={|| log += 1}>go</button>;\n}";
    let components = analyse(source).unwrap();
    let promoted: Vec<&str> = components[0]
        .promoted
        .iter()
        .map(|held| held.name.as_str())
        .collect();
    assert_eq!(promoted, ["log"]);
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

// -------------------------------------- a write through a mut borrow

#[test]
fn a_write_through_a_mutable_borrow_of_state_is_refused() {
    // `clips.get().get_mut(index)` compiles, borrows the copy `get`
    // handed back, changes it, and drops it at the end of the
    // statement. The handler fires and the screen never moves. Found on
    // `apps/video-editor/app/page.rsx`, where four handlers did exactly
    // this.
    let source = "let mut clips: Vec<String> = Vec::new();\n\nfn C() -> Element {\n    return <button onClick={move |_| {\n        if let Some(clip) = clips.get_mut(0) { clip.clear(); }\n    }}>x</button>;\n}\n";
    let said = analyse(source).expect_err("a silent no-op must be refused");
    let text = format!("{said}");
    assert!(text.contains("get_mut"), "got {text}");
    assert!(text.contains("= held;"), "the way out is named: {text}");
}

#[test]
fn an_iter_mut_in_statement_position_is_still_an_update() {
    // The refusal must not swallow the case that already worked: in
    // statement position the whole expression is a write, and `update`
    // is the right shape for it.
    let source = "let mut rows: Vec<String> = Vec::new();\n\nfn C() -> Element {\n    return <button onClick={move |_| {\n        rows.iter_mut().for_each(|row| row.clear());\n    }}>x</button>;\n}\n";
    let components = analyse(source).expect("a statement is a write");
    assert_eq!(
        components[0].promoted[0].mutations[0].kind,
        MutationKind::Method
    );
}

#[test]
fn a_mutable_borrow_of_a_plain_local_is_not_refused() {
    // The copy-change-put-back shape the refusal asks for. `held` is an
    // ordinary local, and nothing about it is state.
    let source = "let mut clips: Vec<String> = Vec::new();\n\nfn C() -> Element {\n    return <button onClick={move |_| {\n        let mut held = clips.get();\n        if let Some(clip) = held.get_mut(0) { clip.clear(); }\n        clips.set(held);\n    }}>x</button>;\n}\n";
    analyse(source).expect("the way out has to compile");
}
