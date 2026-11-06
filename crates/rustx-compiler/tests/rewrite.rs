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
    assert!(
        !rust.contains("let mut n = 0"),
        "the plain local survived: {rust}"
    );
}

#[test]
fn a_compound_mutation_in_a_handler_becomes_an_update() {
    let rust = rewrite_one(COUNTER);
    assert!(rust.contains("update"), "got {rust}");
    assert!(
        !rust.contains("n += 1"),
        "the raw mutation survived: {rust}"
    );
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
    assert!(
        !rust.contains("get () . push"),
        "the push went to a copy: {rust}"
    );
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
    assert_eq!(
        out.rust.lines().count(),
        COUNTER.lines().count(),
        "{}",
        out.rust
    );
}

#[test]
fn a_name_that_appears_in_interface_text_is_not_rewritten() {
    // The state is called `count` and so is a CSS class, and so is a
    // word in the sentence. Only the code may be touched.
    let source = "fn C() -> Element {\n    let mut count = 0;\n    return <div onClick={|| count += 1}><span className=\"count\">{count} count</span></div>;\n}";
    let rust = rewrite_one(source);
    assert!(
        rust.contains(r#""count""#),
        "the class name was rewritten: {rust}"
    );
    assert!(
        !rust.contains("count.get() count"),
        "the sentence was rewritten: {rust}"
    );
    assert!(
        rust.contains("count . get ()"),
        "the real read was missed: {rust}"
    );
}

#[test]
fn a_name_inside_a_string_literal_is_left_alone() {
    let source = "fn C() -> Element {\n    let mut n = 0;\n    return <p title=\"n is here\" onClick={|| n += 1}>{n}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains(r#""n is here""#), "got {rust}");
}

#[test]
fn reading_and_writing_in_one_statement_does_not_corrupt_the_source() {
    // `n = n + 1` is the most natural way to write the document's own
    // example. Two edits used to overlap and produce unparseable Rust.
    let source = "fn C() -> Element {\n    let mut n = 0;\n    return <button onClick={|| n = n + 1}>{n}</button>;\n}";
    let rust = rewrite_one(source);
    assert!(!rust.contains(")() +"), "edits overlapped: {rust}");
    assert!(rust.contains("n . set (n . get () + 1)"), "got {rust}");
}

#[test]
fn a_shadowed_local_is_refused_rather_than_mangled() {
    // Shadowing is idiomatic Rust. Matching occurrences by name alone
    // cannot tell the two apart, so the honest answer is to say so.
    let source = "fn C() -> Element {\n    let mut n = 0;\n    let mut n = 5;\n    return <p onClick={|| n += 1}>{n}</p>;\n}";
    let error = transpile(source).expect_err("shadowing should be refused");
    assert!(error.message.contains("twice"), "got {}", error.message);
    assert_eq!(error.line, 3, "the second declaration should be named");
}

#[test]
fn an_assignment_whose_value_is_a_match_keeps_its_arms() {
    let source = "fn C() -> Element {\n    let mut n = 0;\n    return <p onClick={|| n = match n { 0 => 1, _ => 0 }}>{n}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(!rust.contains("=> 0 } }}"), "the match leaked out: {rust}");
}

#[test]
fn a_mutation_inside_a_nested_closure_keeps_its_punctuation() {
    let source = "fn C() -> Element {\n    let mut n = 0;\n    return <p onClick={|| { [1].iter().for_each(|x| n += x); }}>{n}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(
        !rust.contains("; })"),
        "the `)` and `;` were swapped: {rust}"
    );
    assert!(rust.contains("* __v += x)"), "got {rust}");
}

#[test]
fn a_field_assignment_is_recognised_or_refused_but_never_ignored() {
    // `p.x = 1` used to promote nothing, so the handler borrowed a local
    // that dies with the render and the developer met a borrow error
    // against code they never wrote.
    let source = "fn C() -> Element {\n    let mut p = Point::new();\n    return <p onClick={|| p.x = 1}>{p.x}</p>;\n}";
    match transpile(source) {
        Ok(out) => assert!(
            out.rust.contains("update"),
            "silently ignored: {}",
            out.rust
        ),
        Err(error) => assert!(
            error.message.contains("field"),
            "an unhelpful refusal: {}",
            error.message
        ),
    }
}

#[test]
fn a_method_mutation_that_is_the_whole_handler_still_goes_through_update() {
    // `|| items.push(1)` with no braces and no semicolon: a handler
    // returns nothing, so its body's value is thrown away and the call
    // is a mutation like any other. Read as an expression instead, it
    // became `items.get().push(1)` — a push onto a copy, which compiles
    // and does nothing, and a list that never reordered on screen.
    let source = "fn C() -> Element {\n    let mut items: Vec<u8> = Vec::new();\n    return <button onClick={move |_| items.push(1)}>{items.len()}</button>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("update"), "got {rust}");
    assert!(
        !rust.contains("get().push"),
        "the push went to a copy: {rust}"
    );
    assert!(
        !rust.contains("get () . push"),
        "the push went to a copy: {rust}"
    );
}

#[test]
fn a_method_mutation_alone_in_a_braced_handler_goes_through_update() {
    let source = "fn C() -> Element {\n    let mut items: Vec<u8> = Vec::new();\n    return <button onClick={move |_| { items.push(1) }}>{items.len()}</button>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("update"), "got {rust}");
    assert!(
        !rust.contains("get () . push"),
        "the push went to a copy: {rust}"
    );
}

#[test]
fn a_let_mut_nothing_writes_to_warns_about_nothing() {
    // Half-written state: the value is shown, the button that will
    // change it has not been typed yet. Nothing writes to it, so there
    // is nothing to keep across renders and it stays a plain local —
    // but a plain `let mut` nobody writes to makes `rustc` warn, and the
    // warning points at a generated file in a temporary directory,
    // which is a path the developer has never seen. The framework is
    // what generated it, so the framework is what silences it.
    let source = "fn Page() -> Element {\n    let mut x = 0;\n    return <p>{x}</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("allow (unused_mut)") || rust.contains("allow(unused_mut)"),
        "got {rust}"
    );
}

#[test]
fn a_write_in_an_async_block_promotes_the_local() {
    // The spec names the asynchronous block beside the handler: a local
    // written from one is state, for the same reason and in the same
    // words.
    let rust = rewrite_one(
        "fn C() -> Element {\n    let mut n = 0;\n    task(async move { n += 1; });\n    return <p>{n}</p>;\n}",
    );
    assert!(
        rust.contains("rustx_dom::state::cell"),
        "the local stayed a plain local: {rust}"
    );
}

#[test]
fn a_write_in_an_async_block_goes_through_the_remote() {
    // A cell is an `Rc<RefCell<…>>` on the render thread, and a future
    // runs on the runtime's threads. What crosses is a handle that says
    // where the cell is, and the write lands on a frame.
    let rust = rewrite_one(
        "fn C() -> Element {\n    let mut n = 0;\n    task(async move { n += 1; });\n    return <p>{n}</p>;\n}",
    );
    assert!(rust.contains("n.remote()"), "got {rust}");
    assert!(rust.contains("spawn_future"), "got {rust}");
    assert!(
        rust.contains("n.update("),
        "the write did not become an update: {rust}"
    );
    assert!(
        !rust.contains("n += 1"),
        "a bare write to state survived into the future: {rust}"
    );
}

#[test]
fn a_shared_value_written_in_an_async_block_is_bound_before_the_block() {
    // `let mut rows = …` at the top of a page. The handle is built on
    // the render thread, where the store lives, and moved in.
    let rust = rewrite_one(
        "let mut rows = 0;\n\nfn C() -> Element {\n    task(async move { rows += 1; });\n    return <p>{rows}</p>;\n}",
    );
    assert!(
        rust.contains("rustx_dom::store::shared(\"rows\", || 0).remote()"),
        "got {rust}"
    );
    assert!(rust.contains("rows.update("), "got {rust}");
}

#[test]
fn a_read_of_state_in_an_async_block_is_refused_in_words() {
    // A cell cannot be read from another thread, and pretending
    // otherwise would hand the developer a borrow error about a
    // generated file. Refused where it was written, with the way out.
    let said = transpile(
        "fn C() -> Element {\n    let mut n = 0;\n    task(async move { let copy = n; n += copy; });\n    return <p>{n}</p>;\n}",
    )
    .expect_err("should refuse");
    let said = format!("{said:?}");
    assert!(said.contains("read"), "got {said}");
    assert!(
        said.contains("async") || said.contains("task"),
        "the refusal does not say where the read is: {said}"
    );
}

#[test]
fn an_async_task_that_writes_keeps_the_line_count() {
    let source = "fn C() -> Element {\n    let mut n = 0;\n    task(async move {\n        n += 1;\n    });\n    return <p>{n}</p>;\n}";
    let rust = rewrite_one(source);
    assert_eq!(
        rust.matches('\n').count(),
        source.matches('\n').count(),
        "got {rust}"
    );
}
