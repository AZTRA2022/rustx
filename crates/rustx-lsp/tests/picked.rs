//! The file a question about an expression is asked against.
//!
//! A JSX region is emitted as one long line of Rust, so a column inside
//! it has no counterpart and `rust-analyzer` cannot be asked about it.
//! What it can be asked about is a file where that region is blank
//! *except* for the expression under the cursor, laid at its own line
//! and column — everything else being the real generated code, so the
//! types are real and the positions match character for character.

use rustx_lsp::picked;

/// The text without the `@`, and the one-based line and column it was
/// at.
///
/// `@` and not `|`: a closure's pipes are everywhere in these tests, and
/// a marker that is also code is a marker that eats it.
fn cursor(marked: &str) -> (String, usize, usize) {
    let at = marked.find('@').expect("a test marks the cursor with `@`");
    let before = &marked[..at];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rfind('\n')
        .map_or(before.chars().count(), |newline| {
            before[newline + 1..].chars().count()
        })
        + 1;
    (marked.replace('@', ""), line, column)
}

/// The real generated file for a source, through the real compiler.
fn generated(source: &str) -> String {
    rustx_compiler::transpile::transpile(source)
        .expect("the test's page must compile")
        .rust
}

fn line_of(text: &str, line: usize) -> String {
    text.lines().nth(line - 1).unwrap_or_default().to_string()
}

#[test]
fn the_expression_under_the_cursor_is_where_it_is_in_the_rsx() {
    let marked = "\
fn Page() -> Element {
    let name = \"aztra\".to_string();
    return <main>
        <span>{name.le@}</span>
    </main>;
}
";
    let (source, line, column) = cursor(marked);
    let made = picked::file(&source, &generated(&source), line, column).expect("a file");

    let there = line_of(&made, line);
    let at = there
        .find("name.le")
        .unwrap_or_else(|| panic!("got {there:?}"));
    assert_eq!(
        at + 1,
        column - "name.le".len(),
        "the expression is not at its own column: {there:?}"
    );
    // And what is around it is the real code, which is what makes the
    // types real.
    assert!(
        made.contains("let name = \"aztra\".to_string();"),
        "the function's own body was lost:\n{made}"
    );
}

#[test]
fn the_made_file_has_as_many_lines_as_the_real_one() {
    // A file one line shorter answers one line off, everywhere below.
    let marked = "\
fn Page() -> Element {
    let name = \"aztra\".to_string();
    return <main>
        <span>{na@me}</span>
        <em>after</em>
    </main>;
}
";
    let (source, line, column) = cursor(marked);
    let real = generated(&source);
    let made = picked::file(&source, &real, line, column).expect("a file");
    assert_eq!(
        made.lines().count(),
        real.lines().count(),
        "made:\n{made}\nreal:\n{real}"
    );
}

#[test]
fn the_made_file_is_rust_a_compiler_accepts() {
    // Not valid *typed* Rust — the expression stands where an element
    // was, so its type is wrong, and rust-analyzer completes through a
    // type error without blinking. What it cannot do is complete through
    // a file it cannot parse.
    let marked = "\
fn Page() -> Element {
    let count = 2;
    return <main className=\"x\">
        <button onClick={move |_| count +@ 1}>go</button>
    </main>;
}
";
    let (source, line, column) = cursor(marked);
    let made = picked::file(&source, &generated(&source), line, column).expect("a file");
    syn::parse_file(&made).unwrap_or_else(|error| {
        panic!("rust-analyzer would not parse this:\n{made}\n\n{error}");
    });
}

#[test]
fn a_completion_in_a_handler_sees_the_pages_own_state() {
    // The case a developer meets first: `onClick={move |_| points.@}`.
    // What has to be in the file for that to work is the `let` above,
    // untouched.
    let marked = "\
let mut points = 0;

fn Page() -> Element {
    return <main>
        <button onClick={move |_| points.@}>go</button>
    </main>;
}
";
    let (source, line, column) = cursor(marked);
    let made = picked::file(&source, &generated(&source), line, column).expect("a file");
    let there = line_of(&made, line);
    assert!(
        there.contains("move |_| points."),
        "the handler is not on its own line: {there:?}"
    );
    assert!(
        made.contains("shared(\"points\""),
        "the page's shared state was lost, so the types would be wrong:\n{made}"
    );
}

#[test]
fn an_expression_nobody_closed_still_gets_its_types() {
    // Review Focus 2: `{points.` with no `}` — the state of a file while
    // somebody is typing in it, and the moment completion matters most.
    // The page does not compile, so the generated file is the last one
    // that did; the expression is the one on screen now.
    let marked = "\
let mut points = 0;

fn Page() -> Element {
    return <main>
        <span>{points.@
    </main>;
}
";
    let (source, line, column) = cursor(marked);
    let whole = "\
let mut points = 0;

fn Page() -> Element {
    return <main>
        <span>{points}</span>
    </main>;
}
";
    let made = picked::file(&source, &generated(whole), line, column)
        .expect("a half-typed expression still gets a file");
    assert!(
        line_of(&made, line).contains("points."),
        "got {:?}",
        line_of(&made, line)
    );
}

#[test]
fn a_cursor_that_is_not_in_an_expression_gets_no_file() {
    // Plain Rust is asked about the real generated file, not a made one.
    let marked = "fn Page() -> Element {\n    let x = 1@;\n    return <main />;\n}\n";
    let (source, line, column) = cursor(marked);
    assert!(picked::file(&source, &generated(&source), line, column).is_none());
}

#[test]
fn a_one_line_function_keeps_its_braces() {
    // Blanking a region must not take the `; }` that followed it on the
    // same line: an unbalanced brace is a file rust-analyzer cannot
    // parse, and then nothing works anywhere.
    let marked = "fn Page() -> Element { let n = 1; return <span>{n@}</span>; }\n";
    let (source, line, column) = cursor(marked);
    let made = picked::file(&source, &generated(&source), line, column).expect("a file");
    syn::parse_file(&made).unwrap_or_else(|error| {
        panic!("the braces were lost:\n{made}\n\n{error}");
    });
}
