//! A `let mut` written at the top of a page.
//!
//! The design document's rule is one sentence: "Pas de `use_state`, pas
//! de `signal`. On écrit `let mut n = 0;`, le compilateur fait le
//! reste." A value two distant components share is the same sentence,
//! written one level out — in the file rather than in a component.

use rustx_compiler::transpile::transpile;

fn rewrite_one(source: &str) -> String {
    transpile(source).expect("transpilable").rust
}

const PAGE: &str = "let mut points = 0;\n\n\
     fn Page() -> Element {\n    \
     return <div><p id=\"chart\">{points}</p><button onClick={move |_| points += 1}>add</button></div>;\n\
     }\n";

#[test]
fn a_let_mut_at_the_top_of_a_file_is_shared() {
    let rust = rewrite_one(PAGE);
    assert!(rust.contains("store::shared(\"points\""), "got {rust}");
    assert!(
        !rust.contains("let mut points = 0 ;") && !rust.contains("let mut points = 0;"),
        "the declaration should have become a marker: {rust}"
    );
}

#[test]
fn two_components_see_one_shared_value() {
    let source = "let mut points = 0;\n\n\
         fn Page() -> Element {\n    \
         return <p>{points}</p>;\n}\n\n\
         fn Panel() -> Element {\n    \
         return <button onClick={move |_| points += 1}>add</button>;\n}\n";
    let rust = rewrite_one(source);
    assert_eq!(
        rust.matches("store::shared(\"points\"").count(),
        2,
        "one binding in each of the two components: {rust}"
    );
}

#[test]
fn a_local_of_the_same_name_wins_inside_its_component() {
    // Review Focus 2. A component that declares its own `points` is
    // talking about its own, and nothing it writes may reach the
    // shared one.
    let source = "let mut points = 0;\n\n\
         fn Page() -> Element {\n    \
         let mut points = 5;\n    \
         return <div><p>{points}</p><button onClick={move |_| points += 1}>add</button></div>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("state::cell("),
        "the local should be a cell: {rust}"
    );
    assert!(
        !rust.contains("store::shared(\"points\""),
        "nothing in this component touches the shared value: {rust}"
    );
}

#[test]
fn a_shared_write_in_a_handler_becomes_an_update() {
    let rust = rewrite_one(PAGE);
    assert!(
        rust.contains("points . update (| __v | * __v += 1)"),
        "got {rust}"
    );
}

#[test]
fn a_shared_read_in_the_page_becomes_a_get() {
    let rust = rewrite_one(PAGE);
    assert!(rust.contains("points . get ()"), "got {rust}");
}

#[test]
fn the_file_keeps_its_line_count() {
    let rust = rewrite_one(PAGE);
    assert_eq!(rust.lines().count(), PAGE.lines().count(), "got {rust}");
}

#[test]
fn a_file_with_no_shared_value_is_untouched_by_this() {
    let source = "fn Page() -> Element {\n    return <p>hi</p>;\n}\n";
    assert!(!rewrite_one(source).contains("shared"));
}

#[test]
fn a_shared_write_inside_a_task_reaches_the_value_by_its_name() {
    // A task's body runs on another thread, and what it posts back must
    // be able to cross: a captured handle could not, a type can. So a
    // write inside a `task` reaches the value the way the registry does
    // — by the name it was declared with.
    let source = "let mut ticks = 0;\n\n\
         fn Panel() -> Element {\n    \
         task(move |alive| {\n        \
         while alive.get() {\n            \
         let _ = alive.post(|| ticks += 1);\n        }\n    });\n\n    \
         return <p id=\"panel\">{ticks}</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("shared(\"ticks\", || 0).update"),
        "got {rust}"
    );
    assert!(
        !rust.contains("ticks += 1"),
        "the write should have been rewritten: {rust}"
    );
}

#[test]
fn a_let_mut_written_in_the_text_of_a_page_is_text() {
    // A page that shows code to its reader writes `let mut` in its own
    // interface. It is text, like any other, and declares nothing.
    let source = "fn Page() -> Element {\n    \
         return <pre><span>let mut</span> n = 0;</pre>;\n}\n";
    let rust = rewrite_one(source);
    assert!(!rust.contains("store::shared"), "got {rust}");
}

#[test]
fn a_let_mut_written_in_a_comment_is_a_comment() {
    // A page documents itself, and what a page about state documents is
    // `let mut n = 0;`. Read as a declaration, it was blanked out of the
    // comment — leaving `/// One ``,` — and published as a value of the
    // application under the name `n`.
    let source = "/// One `let mut n = 0;`, as many counters as places.\n\
         fn Page() -> Element {\n    return <p>hi</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(!rust.contains("store::shared"), "got {rust}");
    assert!(
        rust.contains("One `let mut n = 0;`"),
        "the comment was edited: {rust}"
    );
}

#[test]
fn a_let_mut_inside_a_string_literal_is_a_string() {
    let source = "fn Page() -> Element {\n    \
         let shown = \"let mut n = 0;\";\n    return <p>{shown}</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(!rust.contains("store::shared"), "got {rust}");
    assert!(
        rust.contains("let mut n = 0;"),
        "the string was edited: {rust}"
    );
}

#[test]
fn a_shared_value_a_component_only_touches_inside_a_task_still_reaches_it() {
    // The layout of an application counts the times it started its own
    // thread. Nothing shows that value on this screen, so the only use
    // of the name is inside the `task` — and a use the component was not
    // credited with was left as a bare name no scope had.
    let source = "let mut starts = 0;\n\n\
         fn Page() -> Element {\n    \
         task(move |alive| {\n        let _ = alive.post(|| starts += 1);\n    });\n    \
         return <p>hi</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("shared(\"starts\", || 0).update"),
        "got {rust}"
    );
}

#[test]
fn a_local_of_a_plain_function_of_the_page_is_not_shared_state() {
    // A page is allowed to have functions that are not components:
    // cutting a path into segments, formatting a number. Their locals
    // are theirs. "Written at the top level" was read as "not inside a
    // *component*", so a `let mut` inside such a function was taken for
    // a value of the page -- its declaration was blanked where it was
    // written and bound again at the top of every component, and the
    // function came out with its uses naming nothing at all.
    let source = "fn Page() -> Element {\n    \
            let deux = couper(\"a/b\");\n    \
            return <p>{deux.0}</p>;\n\
        }\n\n\
        fn couper(chemin: &str) -> (String, String) {\n    \
            let mut morceaux = chemin.split('/');\n    \
            let un = morceaux.next().unwrap_or_default().to_string();\n    \
            let deux = morceaux.next().unwrap_or_default().to_string();\n    \
            (un, deux)\n\
        }\n";

    let rust = rewrite_one(source);
    assert!(
        rust.contains("let mut morceaux = chemin.split('/')"),
        "the function's own local was taken away: {rust}"
    );
    assert!(
        !rust.contains("shared(\"morceaux\""),
        "a local of a plain function became a value of the page: {rust}"
    );
    assert!(
        syn::parse_file(&rust).is_ok(),
        "what came out does not parse: {rust}"
    );
}

#[test]
fn a_local_of_a_method_in_the_page_is_not_shared_state_either() {
    // The same rule, for the other place a page puts plain Rust.
    let source = "struct Compte { combien: usize }\n\n\
        impl Compte {\n    \
            fn dire(&self) -> String {\n        \
                let mut texte = String::new();\n        \
                texte.push_str(\"x\");\n        \
                texte\n    \
            }\n\
        }\n\n\
        fn Page() -> Element {\n    \
            return <p>{Compte { combien: 1 }.dire()}</p>;\n\
        }\n";

    let rust = rewrite_one(source);
    assert!(
        rust.contains("let mut texte = String::new()"),
        "the method's own local was taken away: {rust}"
    );
    assert!(
        !rust.contains("shared(\"texte\""),
        "a local of a method became a value of the page: {rust}"
    );
}

#[test]
fn the_declared_type_of_a_page_value_reaches_the_store() {
    // `let mut visites: Vec<String> = Vec::new();` became
    // `shared("visites", || Vec::new())`, where the type had nowhere to
    // come from: rustc answered `cannot infer type of the type
    // parameter T`, pointing at a line in a generated file the
    // developer had never written.
    let source = "let mut visites: Vec<String> = Vec::new();\n\n\
         fn Page() -> Element {\n    \
         return <p>{visites.len()}</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("|| -> Vec < String > { Vec :: new() }")
            || rust.contains("|| -> Vec<String> { Vec::new() }"),
        "the declared type should reach the closure: {rust}"
    );
}

#[test]
fn the_declared_type_of_a_component_value_reaches_its_cell() {
    let source = "fn Page() -> Element {\n    \
         let mut choisis: Vec<u32> = Vec::new();\n    \
         return <div><p>{choisis.len()}</p>\
         <button onClick={move |_| choisis.push(1)}>add</button></div>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("|| -> Vec < u32 > { Vec :: new() }")
            || rust.contains("|| -> Vec<u32> { Vec::new() }"),
        "the declared type should reach the cell: {rust}"
    );
}

#[test]
fn a_page_value_only_a_future_writes_and_the_screen_captures_is_still_bound() {
    // The binding was skipped because nothing *outside* the future read
    // it -- and the one read was written in prose, inside a format
    // string, where the test for "is it used here" had never looked.
    // What reached rustc was
    // `format!("{secondes}", secondes = secondes.get())` in a body
    // where `secondes` was never bound.
    let source = "let mut secondes = 0_u32;\n\n\
         fn Page() -> Element {\n    \
         async move {\n        \
         loop {\n            \
         secondes += 1;\n            \
         let instant = format!(\"{secondes}s\");\n            \
         let _ = instant;\n        \
         }\n    \
         };\n\n    \
         return <p>{format!(\"{secondes}\")}</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("let secondes = rustx_dom::store::shared(\"secondes\""),
        "the capture needs a handle to read: {rust}"
    );
}

#[test]
fn a_plain_function_beside_a_page_reaches_its_values() {
    // A helper is ordinary Rust, and a page value has a name: the one
    // thing a developer expects of a name is that it works where it is
    // written. It did not -- rustc answered "cannot find value
    // `journal` in this scope", and helpfully suggested the component
    // `Journal` of the same name.
    let source = "let mut journal: Vec<String> = Vec::new();\n\n\
         fn combien() -> usize {\n    \
         journal.len()\n}\n\n\
         fn Page() -> Element {\n    \
         return <p>{combien()}</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("fn combien() -> usize {let journal = rustx_dom::store::shared(\"journal\""),
        "the helper needs the value bound: {rust}"
    );
    assert!(
        rust.contains("journal.get().len()"),
        "and the read rewritten: {rust}"
    );
}

/// A helper that *writes* a page value.
///
/// The read half of this has worked since the test above. The write half
/// did not, and the shape it broke on is the most ordinary one there is:
/// an event handler that calls `send()`, and `send()` doing the work. A
/// write was only ever rewritten inside a handler's own braces, so a
/// write in a plain function was left exactly as written — against a
/// binding that is now a `Shared<T>`. rustc then answered "expected
/// `Shared<bool>`, found `bool`" and "no method named `push` found for
/// struct `Shared<T>`" about a file in `target/`, naming the line of a
/// generated page the developer has never opened.
#[test]
fn a_plain_function_beside_a_page_writes_its_values() {
    let source = "let mut envoi = false;\nlet mut brouillon = String::new();\n\
         let mut journal: Vec<String> = Vec::new();\n\n\
         fn envoyer() {\n    \
         journal.push(brouillon.clone());\n    \
         brouillon = String::new();\n    \
         envoi = true;\n}\n\n\
         fn Page() -> Element {\n    \
         return <button onClick={move |_| envoyer()}>{envoi}</button>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("journal.update(|__v| { __v.push(brouillon.get().clone()); })"),
        "a method write becomes an update, with the read inside it: {rust}"
    );
    assert!(
        rust.contains("brouillon.set(String::new())"),
        "an assignment becomes a set: {rust}"
    );
    assert!(
        rust.contains("envoi.set(true)"),
        "and so does the one after it: {rust}"
    );
    assert!(
        !rust.contains("envoi = true"),
        "the raw assignment survived: {rust}"
    );
}

/// `let row = rows.get(0);` — a borrow out of a page value.
///
/// `get()` hands back the value and not a borrow of it, so the read is a
/// temporary: the `let` borrowed something that died at the end of its
/// own statement, and rustc said so about a generated file. The clone is
/// bound first now, so the borrow has something to outlive.
#[test]
fn a_let_that_borrows_out_of_a_page_value_gets_the_clone_bound_first() {
    let source = "let mut lignes: Vec<String> = Vec::new();\n\n\
         fn Page() -> Element {\n    \
         let ligne = lignes.first();\n    \
         return <p>{ligne.cloned().unwrap_or_default()}</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("= lignes.get();"),
        "the clone should be bound before the let: {rust}"
    );
    assert!(
        !rust.contains("lignes.get().first()"),
        "the borrow of a temporary survived: {rust}"
    );
    assert_eq!(
        rust.lines().count(),
        source.lines().count(),
        "the hoist must stay on the line it was written on: {rust}"
    );
}

/// And a read that is the whole value is left exactly as it was: there
/// is no temporary to outlive, and a binding would be noise.
#[test]
fn a_read_that_is_the_value_itself_is_not_bound_to_anything() {
    let source = "let mut n = 0;\n\n\
         fn Page() -> Element {\n    \
         let m = n;\n    \
         return <p>{m}</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(rust.contains("let m = n.get();"), "got {rust}");
    assert!(!rust.contains("__r"), "nothing should be hoisted: {rust}");
}

/// The same rewrite must not reach a component's own render.
///
/// A component body is not a place to write state from — the write
/// would run on every render — and the one write that legitimately sits
/// outside a handler there is already handled by `keep`, `effect` and
/// `task`, each rewritten whole. Two edits over the same bytes produce
/// something that is not Rust, which is what this guards.
#[test]
fn a_write_inside_a_kept_call_is_still_that_calls_own_rewrite() {
    let source = "let mut vus = 0;\n\n\
         fn Page() -> Element {\n    \
         effect((), || { vus += 1; });\n    \
         return <p>{vus}</p>;\n}\n";
    let rust = rewrite_one(source);
    assert!(
        rust.contains("effect_at"),
        "the call should still be rewritten whole: {rust}"
    );
    assert_eq!(
        rust.matches(".update(").count(),
        1,
        "the write should be rewritten exactly once: {rust}"
    );
}

#[test]
fn an_async_function_that_reaches_a_page_value_is_refused() {
    // It cannot work and must not look as though it does: a page value
    // lives on the thread that draws, and an `async fn` is run by the
    // runtime, somewhere else. What reached the developer instead was
    // four errors in a generated file.
    let source = "let mut annule = false;\n\n\
         async fn ouvrier() {\n    \
         if annule {\n        \
         return;\n    \
         }\n}\n\n\
         fn Page() -> Element {\n    \
         return <p>{annule}</p>;\n}\n";
    let refusal = transpile(source).expect_err("this cannot be transpiled");
    let said = format!("{refusal:?}");
    assert!(
        said.contains("annule") && said.contains("ouvrier"),
        "the refusal should name the value and the function: {said}"
    );
}

#[test]
fn a_value_declared_over_several_lines_moves_no_line_below_it() {
    // A shared value is relocated to wherever it is reached, once per
    // use, and a declaration written over several lines took its
    // newlines with it: the page below grew by four lines for every
    // read in the `keep`, and every `rustc` diagnostic under the
    // declaration was reported that much further down than it was
    // written.
    let source = "let mut agents: Vec<String> = vec![\n    \
         \"scout\".to_string(),\n    \
         \"closer\".to_string(),\n\
         ];\n\n\
         fn Page() -> Element {\n    \
         keep(async move {\n        \
         let first = agents.len();\n        \
         let again = agents.len();\n        \
         let _ = first + again;\n    \
         });\n    \
         return <p>{agents.len()}</p>;\n}\n";
    let built = transpile(source).expect("transpiles");
    assert_eq!(
        source.lines().count(),
        built.rust.lines().count(),
        "the generated file changed its line count:\n{}",
        built.rust
    );
}

#[test]
fn a_declaration_holding_a_string_with_a_newline_in_it_is_left_as_written() {
    // Collapsing whitespace between tokens changes no program.
    // Collapsing a newline that is part of a string's value changes
    // what the page says, so that one is refused and the text is kept
    // as the developer wrote it — drift and all.
    let source = "let mut banner = \"first\nsecond\".to_string();\n\n\
         fn Page() -> Element {\n    \
         keep(async move {\n        \
         let _ = banner.len();\n    \
         });\n    \
         return <p>{banner.clone()}</p>;\n}\n";
    let built = transpile(source).expect("transpiles");
    assert!(
        built.rust.contains("\"first\nsecond\""),
        "the string was reflowed:\n{}",
        built.rust
    );
}

#[test]
fn a_method_on_a_value_inside_an_expression_is_a_read_not_a_write() {
    // `label.clone()` is the last expression of the `else`, and the
    // `else` is the value of the field. The test for "is this call a
    // statement?" counted brackets and not braces, so that closing
    // brace looked like the handler's own and the call was read as a
    // write: `label.update(|__v| { __v.clone(); })`, which mutates
    // nothing and hands back nothing.
    let source = "let mut label = String::new();
         let mut rows: Vec<String> = Vec::new();

         fn Page() -> Element {
             return <button onClick={move |_| {
                 rows.push(if label.is_empty() { \"none\".to_string() } else { label.clone() });
             }}>push</button>;
}
";
    let built = transpile(source).expect("transpiles");
    // A handler is re-tokenised, so the spacing is the tokeniser's.
    let tight: String = built.rust.chars().filter(|each| !each.is_whitespace()).collect();
    assert!(
        tight.contains("label.get().clone()"),
        "the clone should be a read:
{}",
        built.rust
    );
    assert!(
        !tight.contains("label.update"),
        "the clone was read as a write:
{}",
        built.rust
    );
    // And the rewrite around it is still whole: the nested edit used to
    // be applied into it and left the tail of the original behind.
    assert!(
        tight.contains("rows.update(|__v|{__v.push("),
        "the enclosing rewrite was mangled:
{}",
        built.rust
    );
    assert_eq!(
        tight.matches("cipher_hex").count(),
        0,
        "nothing of the original should be left over:
{}",
        built.rust
    );
}

#[test]
fn a_method_that_is_a_statement_of_its_own_is_still_a_write() {
    // The counterpart: nothing above may turn a real write into a read.
    let source = "let mut rows: Vec<String> = Vec::new();

         fn Page() -> Element {
             return <button onClick={move |_| {
                 if true { rows.push(\"one\".to_string()); }
                 rows.push(\"two\".to_string());
             }}>push</button>;
}
";
    let built = transpile(source).expect("transpiles");
    let tight: String = built.rust.chars().filter(|each| !each.is_whitespace()).collect();
    assert_eq!(
        tight.matches("rows.update(|__v|{__v.push(").count(),
        2,
        "both pushes should be writes:
{}",
        built.rust
    );
}

/// A page value read in the `else` of a `let`, which is a read.
///
/// The test for "is this method call a statement, and so a write?"
/// walked forward from the call looking for the first `;` or `}` at
/// depth zero, and read a `}` followed by `;` as "the block was a
/// statement". But a statement block takes no semicolon in Rust: a `};`
/// is almost always a `let` whose value is a block. So
///
/// ```ignore
/// let found: Vec<Unit> = if needle.is_empty() {
///     Vec::new()
/// } else {
///     departments.iter().filter(…).cloned().collect()
/// };
/// ```
///
/// had `.collect()` read as a write to `departments`, and the page was
/// refused with "`departments` is written while `Page` is being drawn"
/// — a sentence about a line that only ever reads. Found on
/// `apps/utility-hub/app/search/page.rsx`.
#[test]
fn a_page_value_read_in_the_else_of_a_let_is_not_a_write() {
    let source = "let mut departments: Vec<String> = Vec::new();\n\
         \n\
         fn Page() -> Element {\n    \
         let needle = String::new();\n    \
         let units: Vec<String> = if needle.is_empty() {\n        \
         Vec::new()\n    \
         } else {\n        \
         departments.iter().filter(|each| each.contains(&needle)).cloned().collect()\n    \
         };\n    \
         return <p>{units.len()}</p>;\n\
         }\n";
    let built = transpile(source).expect("a page that only reads is not refused");
    let tight: String = built
        .rust
        .chars()
        .filter(|each| !each.is_whitespace())
        .collect();
    assert!(
        tight.contains("departments.get().iter()"),
        "the read should be a read:\n{}",
        built.rust
    );
    assert!(
        !tight.contains("departments.update"),
        "the read was taken for a write:\n{}",
        built.rust
    );
}

/// The same shape, with the block as the value of an assignment rather
/// than of a `let`.
#[test]
fn a_page_value_read_in_a_block_assigned_to_something_is_not_a_write() {
    let source = "let mut rows: Vec<String> = Vec::new();\n\
         \n\
         fn Page() -> Element {\n    \
         let mut seen = 0_usize;\n    \
         let total = match rows.len() {\n        \
         0 => 0,\n        \
         _ => {\n            \
         rows.iter().filter(|each| !each.is_empty()).count()\n        \
         }\n    \
         };\n    \
         return <button onClick={move |_| seen += total}>{seen}</button>;\n\
         }\n";
    let built = transpile(source).expect("transpiles");
    let tight: String = built
        .rust
        .chars()
        .filter(|each| !each.is_whitespace())
        .collect();
    assert!(
        !tight.contains("rows.update"),
        "a read inside a match arm feeding a `let` was taken for a write:\n{}",
        built.rust
    );
}

/// And the counterpart: a real write at the end of a block that *is* a
/// statement is still a write.
///
/// `if …  { }` and `match … { }` written as statements are the two
/// shapes a handler uses to decide between two writes, and a fix that
/// turned either into a read would silently stop the screen changing.
#[test]
fn a_write_that_ends_a_statement_block_is_still_a_write() {
    let source = "let mut rows: Vec<String> = Vec::new();\n\
         \n\
         fn Page() -> Element {\n    \
         return <button onClick={move |_| {\n        \
         if rows.is_empty() { rows.push(\"first\".to_string()) }\n        \
         match rows.len() { 0 => {}, _ => { rows.push(\"more\".to_string()) } }\n    \
         }}>push</button>;\n\
         }\n";
    let built = transpile(source).expect("transpiles");
    let tight: String = built
        .rust
        .chars()
        .filter(|each| !each.is_whitespace())
        .collect();
    assert_eq!(
        tight.matches("rows.update(|__v|{__v.push(").count(),
        2,
        "both pushes should still be writes:\n{}",
        built.rust
    );
}

/// A `;` inside a string literal does not begin a statement.
///
/// The whole class of defect this subsystem keeps producing is a scan
/// that is not aware of what it is scanning over. The head of a
/// statement is read from the statement's start, and finding that start
/// means stepping over strings, raw strings, character literals and
/// both kinds of comment on the way.
#[test]
fn a_semicolon_in_a_string_does_not_begin_a_statement() {
    let source = "let mut rows: Vec<String> = Vec::new();\n\
         \n\
         fn Page() -> Element {\n    \
         let joined = if rows.is_empty() {\n        \
         String::new()\n    \
         } else {\n        \
         rows.join(\"; \")\n    \
         };\n    \
         return <p>{joined}</p>;\n\
         }\n";
    let built = transpile(source).expect("transpiles");
    assert!(
        !built.rust.contains("rows.update"),
        "the `;` inside the separator was read as code:\n{}",
        built.rust
    );
}

/// A page value read in the `else` of an `if` **written inside the
/// markup**.
///
/// The `{…}` of a tag holds an expression, so everything in it is a
/// value — but the braces look exactly like a block's, and the `if`
/// inside them looks exactly like an `if` statement. Reading the head of
/// the statement and stopping at the first construct keyword was
/// therefore not enough: `if status.is_empty() { … } else { … }` begins
/// a statement *at the brace level it is in*, and that brace level is a
/// tag's expression, whose value is used.
///
/// Found on `apps/data-intel/app/console/page.rsx`, which was refused
/// with "`status` is written while `Page` is being drawn" for a line
/// that only reads.
#[test]
fn a_page_value_read_in_an_if_inside_the_markup_is_not_a_write() {
    let source = "let mut status: String = String::new();\n\
         \n\
         fn Page() -> Element {\n    \
         return <p>{if status.is_empty() {\n        \
         \"nothing yet\".to_string()\n    \
         } else {\n        \
         status.clone()\n    \
         }}</p>;\n\
         }\n";
    let built = transpile(source).expect("a page that only reads is not refused");
    let tight: String = built
        .rust
        .chars()
        .filter(|each| !each.is_whitespace())
        .collect();
    assert!(
        tight.contains("status.get().clone()"),
        "the read should be a read:\n{}",
        built.rust
    );
    assert!(
        !tight.contains("status.update"),
        "the read was taken for a write:\n{}",
        built.rust
    );
}

/// The counterpart one level deeper: a construct that is the last thing
/// in a block whose own value is used.
#[test]
fn a_page_value_read_in_a_match_inside_the_markup_is_not_a_write() {
    let source = "let mut rows: Vec<String> = Vec::new();\n\
         \n\
         fn Page() -> Element {\n    \
         return <p>{match rows.len() {\n        \
         0 => \"none\".to_string(),\n        \
         _ => {\n            \
         rows.join(\", \")\n        \
         }\n    \
         }}</p>;\n\
         }\n";
    let built = transpile(source).expect("a page that only reads is not refused");
    assert!(
        !built.rust.contains("rows.update"),
        "a read in a match arm inside the markup was taken for a write:\n{}",
        built.rust
    );
}

/// And a real write that ends an `if` which is itself the last statement
/// of a handler is still a write.
#[test]
fn a_write_ending_the_last_construct_of_a_handler_is_still_a_write() {
    let source = "let mut rows: Vec<String> = Vec::new();\n\
         \n\
         fn Page() -> Element {\n    \
         return <button onClick={move |_| {\n        \
         if rows.is_empty() { rows.push(\"first\".to_string()) }\n    \
         }}>push</button>;\n\
         }\n";
    let built = transpile(source).expect("transpiles");
    let tight: String = built
        .rust
        .chars()
        .filter(|each| !each.is_whitespace())
        .collect();
    assert!(
        tight.contains("rows.update(|__v|{__v.push("),
        "the push should still be a write:\n{}",
        built.rust
    );
}

/// A page value read in the `if` half of an `if/else` written inside the
/// markup.
///
/// The counterpart of the test above, and a second defect: the test for
/// "is the construct the last thing in its block?" looked at the token
/// after the **block**, and the token after the `if`'s block is `else`.
/// An `else` is the same `if` carrying on, not the next statement, so
/// `{if ready { status.clone() } else { … }}` had the `.clone()` read as
/// a write. Found on `apps/data-intel/app/contact/page.rsx`.
#[test]
fn a_page_value_read_in_the_if_half_inside_the_markup_is_not_a_write() {
    let source = "let mut status: String = String::new();\n\
         \n\
         fn Page() -> Element {\n    \
         let ready = true;\n    \
         return <p>{if ready { status.clone() } else { \"not yet\".to_string() }}</p>;\n\
         }\n";
    let built = transpile(source).expect("a page that only reads is not refused");
    let tight: String = built
        .rust
        .chars()
        .filter(|each| !each.is_whitespace())
        .collect();
    assert!(
        tight.contains("status.get().clone()"),
        "the read should be a read:\n{}",
        built.rust
    );
    assert!(
        !tight.contains("status.update"),
        "the read was taken for a write:\n{}",
        built.rust
    );
}

/// And an `else if` chain, which is the same question one link longer.
#[test]
fn a_page_value_read_in_an_else_if_chain_inside_the_markup_is_not_a_write() {
    let source = "let mut status: String = String::new();\n\
         \n\
         fn Page() -> Element {\n    \
         let n = 2;\n    \
         return <p>{if n == 0 { status.clone() } else if n == 1 { status.to_uppercase() } \
         else { String::new() }}</p>;\n\
         }\n";
    let built = transpile(source).expect("a page that only reads is not refused");
    assert!(
        !built.rust.contains("status.update"),
        "a read in an else-if chain was taken for a write:\n{}",
        built.rust
    );
}

/// The counterpart: an `if/else` of two writes, written as a statement
/// of a handler, is still two writes.
#[test]
fn an_if_else_of_two_writes_in_a_handler_is_still_two_writes() {
    let source = "let mut rows: Vec<String> = Vec::new();\n\
         \n\
         fn Page() -> Element {\n    \
         let ready = true;\n    \
         return <button onClick={move |_| {\n        \
         if ready { rows.push(\"yes\".to_string()) } else { rows.push(\"no\".to_string()) }\n    \
         }}>push</button>;\n\
         }\n";
    let built = transpile(source).expect("transpiles");
    let tight: String = built
        .rust
        .chars()
        .filter(|each| !each.is_whitespace())
        .collect();
    assert_eq!(
        tight.matches("rows.update(|__v|{__v.push(").count(),
        2,
        "both pushes should be writes:\n{}",
        built.rust
    );
}
