//! Ordinary Rust before the `return`.
//!
//! A component is a function, and what a function does before it returns
//! is Rust's business: a `for` that fills a list, a `while` that walks
//! one, a `loop` that breaks. Each of them writes a `<` or a `{` that the
//! scanner has to read as what it is, and the scanner is a hand-written
//! one — so each of them is a test.
//!
//! Every one of these runs on a thread with a deadline: the failure a
//! hand-written scanner has is not a wrong answer, it is no answer at
//! all, and a test that hangs is a suite that never finishes.

use std::sync::mpsc;
use std::time::Duration;

use rustx_compiler::transpile::transpile;

/// Transpile, or say which it was: refused, panicked, or never finished.
fn compiled(source: &'static str) -> Result<String, String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(|| transpile(source));
        let _ = sender.send(match outcome {
            Ok(Ok(out)) => Ok(out.rust),
            Ok(Err(error)) => Err(format!("refused: {}", error.message)),
            Err(_) => Err("the compiler panicked".to_string()),
        });
    });
    receiver
        .recv_timeout(Duration::from_secs(10))
        .unwrap_or_else(|_| Err("the compiler never finished".to_string()))
}

#[test]
fn a_for_loop_before_the_return_compiles() {
    let rust = compiled(
        "fn Page() -> Element {\n\
         \x20   let mut rows = Vec::new();\n\
         \x20   for at in 0..5 {\n\
         \x20       rows.push(at.to_string());\n\
         \x20   }\n\
         \x20   return <ul>{rows.len()}</ul>;\n\
         }\n",
    )
    .expect("a for loop before a return");
    assert!(rust.contains("for at in 0..5"), "got {rust}");
}

#[test]
fn a_while_loop_whose_condition_holds_a_less_than_compiles() {
    // `while at < 5` writes a `<` in code, before a `return` that writes
    // one in a tag. A scanner that reads the first as a tag reads the
    // rest of the file as its contents.
    let rust = compiled(
        "fn Page() -> Element {\n\
         \x20   let mut at = 0;\n\
         \x20   while at < 5 {\n\
         \x20       at += 1;\n\
         \x20   }\n\
         \x20   return <p>{at}</p>;\n\
         }\n",
    )
    .expect("a while loop before a return");
    assert!(rust.contains("while at < 5"), "got {rust}");
}

#[test]
fn a_loop_with_a_break_before_the_return_compiles() {
    let rust = compiled(
        "fn Page() -> Element {\n\
         \x20   let mut at = 0;\n\
         \x20   loop {\n\
         \x20       at += 1;\n\
         \x20       if at > 3 { break; }\n\
         \x20   }\n\
         \x20   return <p>{at}</p>;\n\
         }\n",
    )
    .expect("a loop before a return");
    assert!(rust.contains("break"), "got {rust}");
}

#[test]
fn a_loop_that_builds_the_children_compiles() {
    // The shape a page actually has: the loop fills a list of elements
    // and the tag below shows them.
    let rust = compiled(
        "fn Page() -> Element {\n\
         \x20   let mut rows = Vec::new();\n\
         \x20   for at in 0..3 {\n\
         \x20       rows.push(<li>{at}</li>);\n\
         \x20   }\n\
         \x20   return <ul>{rows}</ul>;\n\
         }\n",
    )
    .expect("a loop that builds elements");
    assert!(rust.contains("\"li\"") || rust.contains("li"), "got {rust}");
}

#[test]
fn a_comparison_of_a_generic_before_the_return_compiles() {
    // `Vec<String>` before a `return`: angle brackets that are types.
    let rust = compiled(
        "fn Page() -> Element {\n\
         \x20   let names: Vec<String> = Vec::new();\n\
         \x20   return <p>{names.len()}</p>;\n\
         }\n",
    )
    .expect("a generic before a return");
    assert!(rust.contains("Vec"), "got {rust}");
}

#[test]
fn writing_shared_state_in_the_body_is_refused_in_words() {
    // The shape that froze a window: a loop before the `return` that
    // writes the page's own state. Nothing rewrote it, so what reached
    // rustc was `total.get() += 1` — an assignment to a copy, in a file
    // the developer has never opened. And were it rewritten, a write
    // during a render would ask for another render, for ever.
    let error = compiled(
        "let mut total = 0;\n\
         \n\
         fn Page() -> Element {\n\
         \x20   for at in 0..3 {\n\
         \x20       total += 1;\n\
         \x20   }\n\
         \x20   return <p>{total}</p>;\n\
         }\n",
    )
    .expect_err("a write during a render");
    assert!(
        error.contains("refused:") && error.contains("task") && error.contains("render"),
        "an unhelpful refusal: {error}"
    );
}

#[test]
fn writing_a_component_s_own_state_in_the_body_is_refused_in_words() {
    let error = compiled(
        "fn Page() -> Element {\n\
         \x20   let mut n = 0;\n\
         \x20   for at in 0..3 {\n\
         \x20       n += 1;\n\
         \x20   }\n\
         \x20   return <button onClick={move |_| n += 1}>{n}</button>;\n\
         }\n",
    )
    .expect_err("a write during a render");
    assert!(error.contains("refused:"), "got {error}");
}

#[test]
fn a_loop_that_writes_a_plain_local_is_left_alone() {
    // The same loop over a variable no handler writes: an ordinary Rust
    // local, an ordinary Rust loop, and nothing to refuse.
    let rust = compiled(
        "fn Page() -> Element {\n\
         \x20   let mut total = 0;\n\
         \x20   for at in 0..3 {\n\
         \x20       total += at;\n\
         \x20   }\n\
         \x20   return <p>{total}</p>;\n\
         }\n",
    )
    .expect("a plain local in a loop");
    assert!(rust.contains("total += at"), "got {rust}");
}
