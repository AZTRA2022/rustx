//! The examples in the README, compiled.
//!
//! A documentation example that does not compile is a bug, and the only
//! way to know is to run the compiler over it. These are `.rsx`
//! fragments, so what runs is the `.rsx` compiler — not `rustc`: the
//! examples name functions they do not define (`charger`, `chercher`),
//! which is the right thing for a page of documentation and the wrong
//! thing for a type-checker.

/// Every fenced `rust` block of the README, in order.
fn examples() -> Vec<String> {
    let readme = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md"),
    )
    .expect("the README is beside the crates");

    let mut found = Vec::new();
    let mut current: Option<String> = None;
    for line in readme.lines() {
        match (line.trim_end(), current.as_mut()) {
            ("```rust", None) => current = Some(String::new()),
            ("```", Some(_)) => found.push(current.take().expect("a block")),
            (_, Some(block)) => {
                block.push_str(line);
                block.push('\n');
            }
            _ => {}
        }
    }
    found
}

#[test]
fn the_readme_has_examples_to_check() {
    // A test that found nothing to look at passes for the wrong reason.
    let examples = examples();
    assert!(
        examples.len() > 10,
        "only {} fenced rust blocks were found in the README",
        examples.len()
    );
}

#[test]
fn every_async_example_in_the_readme_compiles() {
    // A bare `async` block is the form the README documents now, and
    // `task(async …)` is still the explicit one. Both are collected,
    // because both have to keep compiling.
    let waiting: Vec<String> = examples()
        .into_iter()
        .filter(|block| block.contains("task(async") || block.contains("async move {"))
        .collect();
    assert!(
        waiting.len() >= 3,
        "the README documents an `async` block in {} examples",
        waiting.len()
    );

    for example in &waiting {
        let built = rustx_compiler::transpile::transpile(example)
            .unwrap_or_else(|refused| panic!("the README says:\n{example}\nand rustx says:\n{refused:?}"));
        assert!(
            built.rust.contains("spawn_future"),
            "an `async` example did not become a future:\n{example}\n{}",
            built.rust
        );
    }
}

#[test]
fn the_readme_promises_no_async_that_is_not_there() {
    // The sentence that used to say the plan was written and the code
    // was not. It is written now, so the sentence has to be gone.
    let readme = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md"),
    )
    .expect("the README is beside the crates");
    assert!(
        !readme.contains("n'est **pas** encore"),
        "the README still says the asynchronous half is missing"
    );
}

#[test]
fn the_awaited_example_in_the_readme_compiles() {
    // The section that says `let x = travail().await;` works in a
    // component. If it stops compiling, the README is lying in the one
    // place somebody would copy from.
    let shown: Vec<String> = examples()
        .into_iter()
        // In a component, which is the claim. `async fn main` awaits
        // too, and it is the section above.
        .filter(|block| block.contains("-> Element") && block.contains(".await"))
        .filter(|block| !block.contains("async move {"))
        .collect();
    assert!(
        !shown.is_empty(),
        "the README no longer shows a value awaited in a component"
    );

    for example in &shown {
        let built = rustx_compiler::transpile::transpile(example).unwrap_or_else(|refused| {
            panic!("the README says:
{example}
and rustx says:
{refused:?}")
        });
        assert!(
            built.rust.contains("task::awaited"),
            "an awaited example did not move off the render thread:
{example}
{}",
            built.rust
        );
    }
}
