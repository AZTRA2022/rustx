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
