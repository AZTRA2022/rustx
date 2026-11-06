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
    assert!(rust.contains("n.remote_until(&__cancel)"), "got {rust}");
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
        rust.contains("rustx_dom::store::shared(\"rows\", || 0).remote_until(&__cancel)"),
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

#[test]
fn a_write_with_no_semicolon_in_an_async_block_is_a_write_and_not_a_read() {
    // `uses_in` hardcoded `tail_discarded = false`, so a method write
    // that ends the block without a semicolon was classified as a read
    // and the file was refused -- telling the developer to move a read
    // that does not exist. `spawn_future` wants `Output = ()`, so an
    // async block's tail value is discarded by construction.
    let rust = rewrite_one(
        "let mut rows = Vec::<u32>::new();\n\nfn C() -> Element {\n    task(async move { rows.push(1) });\n    return <p>{rows.len()}</p>;\n}",
    );
    assert!(rust.contains("rows.update("), "got {rust}");
}

#[test]
fn a_write_in_an_async_block_leaves_what_it_used_borrowable_afterwards() {
    // `move |__v| __v.push(reponse)` took ownership of everything the
    // expression mentioned, so a line after it that used the same value
    // was a use-after-move -- reported against a generated file, for
    // code that would have compiled had the target been a plain local.
    // The value is bound first and only the binding is moved.
    let rust = rewrite_one(
        "let mut lignes = Vec::<String>::new();\n\nfn C() -> Element {\n    task(async move { let r = charger().await; lignes.push(r.clone()); journal(&r); });\n    return <p>{lignes.len()}</p>;\n}",
    );
    assert!(
        rust.contains("let __a0 = r.clone();"),
        "the argument was not bound before the closure: {rust}"
    );
    assert!(
        rust.contains("__v.push(__a0)"),
        "the closure did not take the binding: {rust}"
    );
    assert!(rust.contains("journal(&r)"), "got {rust}");
}

#[test]
fn an_async_task_binds_a_token_its_writes_are_refused_by() {
    // Abandoning a future stops it making new writes. The ones already
    // posted are refused by a token the handles share with the `Task`.
    let rust = rewrite_one(
        "let mut rows = Vec::<u32>::new();\n\nfn C() -> Element {\n    task(async move { rows.push(1); });\n    return <p>{rows.len()}</p>;\n}",
    );
    assert!(rust.contains("Cancel::new()"), "got {rust}");
    assert!(rust.contains(".remote_until(&"), "got {rust}");
    assert!(rust.contains("spawn_future_until("), "got {rust}");
}

// --- Anything a crate's macro writes, inside a page -----------------
//
// "It should work for any crate that uses a macro or whatever." A page
// is Rust, so a page may call a macro, derive a trait, or carry an
// attribute — and the `.rsx` compiler scans braces to find the JSX, so
// a macro with a brace body is exactly where a brace scanner goes wrong.

#[test]
fn a_macro_with_a_brace_body_inside_an_async_block_is_left_alone() {
    // `tokio::select! { … => … }`. The arms are braces that are not JSX
    // and not a closure, inside an argument the compiler rewrites.
    let source = "let mut n = 0;\n\nfn C() -> Element {\n    task(async move {\n        rustx::tokio::select! {\n            _ = tick() => { n += 1; }\n            _ = stop() => {}\n        }\n    });\n    return <p>{n}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("select!"), "the macro call was mangled: {rust}");
    assert!(rust.contains("n.update("), "the write inside an arm was missed: {rust}");
    assert_eq!(
        rust.matches('\n').count(),
        source.matches('\n').count(),
        "got {rust}"
    );
}

#[test]
fn a_derive_and_an_attribute_in_a_page_survive() {
    let source = "#[derive(Debug, Clone, PartialEq)]\npub struct Point {\n    pub at: f64,\n}\n\n#[allow(dead_code)]\nfn aside() -> u8 { 1 }\n\nfn C() -> Element {\n    return <p>ok</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("#[derive(Debug, Clone, PartialEq)]"), "got {rust}");
    assert!(rust.contains("#[allow(dead_code)]"), "got {rust}");
}

#[test]
fn a_macro_that_takes_braces_in_the_body_of_a_component_is_left_alone() {
    // `vec![]`, `format!` with its own braces, and a declarative macro
    // whose body is a block — all before the `return`.
    let source = "fn C() -> Element {\n    let rows = vec![1, 2, 3];\n    let said = format!(\"{} rows\", rows.len());\n    let _held = matches!(rows.len(), 3);\n    return <p>{said}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("vec![1, 2, 3]"), "got {rust}");
    assert!(rust.contains("format!(\"{} rows\""), "got {rust}");
}

#[test]
fn an_applications_own_async_fn_is_awaited_from_a_page() {
    // An `async fn` written anywhere in the crate, awaited in a page.
    // Nothing about it is the framework's business — which is the claim.
    let source = "let mut charge = 0;\n\nfn C() -> Element {\n    task(async move {\n        let got = crate::charger().await;\n        charge += got;\n    });\n    return <p>{charge}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("crate::charger().await"), "got {rust}");
    assert!(rust.contains("charge.update("), "got {rust}");
}

#[test]
fn a_bare_async_block_in_a_component_is_a_task() {
    // JSX asks for no wrapper, so neither does this: an `async` block
    // written as a statement of a component is work that runs beside
    // the render, and `task(...)` around it said nothing the `async`
    // did not.
    let source = "let mut n = 0;\n\nfn C() -> Element {\n    async move {\n        n += 1;\n    };\n    return <p>{n}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("spawn_future_until("), "the block was not spawned: {rust}");
    assert!(rust.contains(".remote_until(&__cancel)"), "no handle was built for it: {rust}");
    assert!(rust.contains("n.update("), "the write inside it was missed: {rust}");
    assert_eq!(
        rust.matches('\n').count(),
        source.matches('\n').count(),
        "got {rust}"
    );
}

#[test]
fn an_async_block_that_is_a_value_is_left_alone() {
    // `let held = async { … };` is a future the component holds, and
    // whatever it does with it is its business. Only a block written as
    // a statement is work to run.
    let source = "fn C() -> Element {\n    let held = async { 1u8 };\n    let _ = held;\n    return <p>ok</p>;\n}";
    let rust = rewrite_one(source);
    assert!(
        !rust.contains("spawn_future"),
        "a future the component holds was spawned behind its back: {rust}"
    );
    assert!(rust.contains("let held = async { 1u8 };"), "got {rust}");
}

#[test]
fn an_async_block_passed_to_something_is_left_alone() {
    let source = "fn C() -> Element {\n    let _ = taken(async { 1u8 });\n    return <p>ok</p>;\n}";
    let rust = rewrite_one(source);
    assert!(
        !rust.contains("spawn_future"),
        "an argument was spawned: {rust}"
    );
}

#[test]
fn task_around_an_async_block_still_means_the_same_thing() {
    // The explicit form keeps working, and spawns once -- not twice.
    let source = "let mut n = 0;\n\nfn C() -> Element {\n    task(async move {\n        n += 1;\n    });\n    return <p>{n}</p>;\n}";
    let rust = rewrite_one(source);
    assert_eq!(
        rust.matches("spawn_future_until(").count(),
        1,
        "got {rust}"
    );
}

#[test]
fn a_bare_async_block_behind_a_comment_is_still_a_statement() {
    // What sits in front of a block is almost always a line saying what
    // it is for. Reading only the whitespace back made every explained
    // block look like the middle of an expression.
    let source = "let mut n = 0;\n\nfn C() -> Element {\n    // What this does, and the url \"http://example\" in it.\n    async move {\n        n += 1;\n    };\n    return <p>{n}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("spawn_future_until("), "got {rust}");
}

#[test]
fn an_await_before_the_return_does_not_block_the_render() {
    // A component is asked for the screen as it is now. What it waits
    // for, it waits for beside the render: the first frame gets nothing
    // and draws nothing, the frame after the future lands gets the
    // value, and the window never stopped.
    let source = "fn C() -> Element {\n    let greeting = fetch().await;\n    return <p>{greeting}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("rustx_engine::task::awaited("),
        "the await was left on the render thread: {rust}"
    );
    assert!(rust.contains("let Some(greeting)"), "got {rust}");
    assert!(rust.contains("else { return Element::Empty; }"), "got {rust}");
    assert!(rust.contains("fetch()"), "the expression was lost: {rust}");
    // The await is inside an `async move` block, which is the only
    // place it can be: the component itself stayed synchronous.
    assert!(
        rust.contains("async move { __work.await }"),
        "the await was not moved into a future: {rust}"
    );
    assert_eq!(
        rust.matches('\n').count(),
        source.matches('\n').count(),
        "got {rust}"
    );
}

#[test]
fn an_awaited_value_that_is_mutable_stays_mutable() {
    let source = "fn C() -> Element {\n    let mut rows = load().await;\n    rows.push(1);\n    return <p>{rows.len()}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("let Some(mut rows)"), "got {rust}");
}

#[test]
fn an_awaited_value_keeps_the_type_it_was_written_with() {
    let source = "fn C() -> Element {\n    let name: String = ask().await;\n    return <p>{name}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("let Some(name): Option<String>"), "got {rust}");
}

#[test]
fn an_awaited_expression_can_read_state() {
    // `fetch(id)` where `id` is a cell: the expression is left where it
    // was written, so the read in it is rewritten like any other.
    let source = "let mut id = 0;\n\nfn C() -> Element {\n    let got = fetch(id).await;\n    return <p onClick={move |_| id += 1}>{got}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("fetch(id.get())"), "the read was missed: {rust}");
    assert!(rust.contains("rustx_engine::task::awaited("), "got {rust}");
}

#[test]
fn an_await_inside_an_async_block_is_left_alone() {
    let source = "let mut n = 0;\n\nfn C() -> Element {\n    async move {\n        tick().await;\n        n += 1;\n    };\n    return <p>{n}</p>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("tick().await"), "got {rust}");
    assert!(!rust.contains("task::awaited("), "got {rust}");
}

#[test]
fn an_await_that_is_not_the_value_of_a_let_is_refused_in_words() {
    let source = "fn C() -> Element {\n    ping().await;\n    return <p>ok</p>;\n}";
    let error = rustx_compiler::transpile::transpile(source).expect_err("refused");
    assert!(
        error.message.contains("await") && error.message.contains("async"),
        "got {error:?}"
    );
}

#[test]
fn work_after_the_await_goes_with_it() {
    // `.await.unwrap_or_else(…)` and `.await.ok()` are how anybody
    // writes this. The tail runs on the answer, which only exists
    // beside the window -- so it travels with the future rather than
    // being left on the render thread.
    let source = "fn C() -> Element {
    let texte = read(\"x\").await.unwrap_or_default();
    return <p>{texte}</p>;
}";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("async move { __work.await.unwrap_or_default() }"),
        "got {rust}"
    );
    assert!(rust.contains("let Some(texte)"), "got {rust}");
    assert_eq!(
        rust.matches('\n').count(),
        source.matches('\n').count(),
        "got {rust}"
    );
}

#[test]
fn an_await_buried_in_an_argument_is_refused_in_words() {
    // `f(g().await)` waits for `g` before `f` is called, which is a wait
    // on the render thread whatever is done with the result.
    let source = "fn C() -> Element {
    let got = wrap(inner().await);
    return <p>{got}</p>;
}";
    let error = rustx_compiler::transpile::transpile(source).expect_err("refused");
    assert!(error.message.contains("await"), "got {error:?}");
}

#[test]
fn a_read_of_state_after_the_await_is_refused_in_words() {
    // What follows the await runs beside the window. A cell is not
    // there, so this is the same refusal an `async` block gets.
    let source = "let mut n = 0;

fn C() -> Element {
    let got = load().await.max(n);
    return <p onClick={move |_| n += 1}>{got}</p>;
}";
    let error = rustx_compiler::transpile::transpile(source).expect_err("refused");
    assert!(
        error.message.contains("after an `.await`"),
        "got {error:?}"
    );
}

#[test]
fn an_accent_in_front_of_a_block_is_not_cut_in_half() {
    // The offsets here are bytes and the text is French: a comment
    // ending on an accented word put a slice boundary inside a
    // character, and the compiler panicked instead of compiling.
    let source = "let mut n = 0;

fn C() -> Element {
    // Le `async` le disait déjà.
    async move {
        n += 1;
    };
    return <p>{n}</p>;
}";
    let rust = rewrite_one(source);
    assert!(rust.contains("spawn_future_until("), "got {rust}");
}

#[test]
fn a_refusal_is_one_sentence_of_prose_and_not_a_wall_of_spaces() {
    // A refusal is read by a developer in a terminal. A line continued
    // in the source carries a `\` or the indentation of the next line
    // lands in the message itself, and what they read is
    // "A value that            comes from a future".
    let walls = [
        "fn C() -> Element {\n    ping().await;\n    return <p>ok</p>;\n}",
        "fn C() -> Element {
    let mut compte = load().await;
    return <p onClick={move |_| compte += 1}>{compte}</p>;
}",
    ];
    for source in walls {
        let error = rustx_compiler::transpile::transpile(source).expect_err("refused");
        assert!(
            !error.message.contains("  "),
            "two spaces in a row: {:?}",
            error.message
        );
    }
}
