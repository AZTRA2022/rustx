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
    assert!(
        rust.contains("store::shared::<__shared_points"),
        "got {rust}"
    );
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
        rust.matches("__shared_points").count(),
        3,
        "the marker, and one binding in each of the two components: {rust}"
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
        !rust.contains("shared::<__shared_points"),
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
