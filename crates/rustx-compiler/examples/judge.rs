//! `cargo run -p rustx-compiler --example judge -- <fichier.rsx> …`
//!
//! Transpile a page and say what happened, without a window, without a
//! runtime and without cargo building an application around it.
//!
//! Why it exists: a page that the attribute macro refuses reports its
//! refusal through `#[rustx::main]`, which means a full application
//! build to see one message — and a second, confusing error about
//! `main` not being allowed to be `async`, because the macro bailed
//! before taking the `async` off. Bisecting a parser bug that way costs
//! ten seconds a try. This costs none, and prints the line and column
//! the compiler points at, with the line itself under it.

use std::path::Path;

fn main() {
    let mut files: Vec<String> = std::env::args().skip(1).collect();
    // `--rust` also prints what was emitted: a page can be accepted and
    // still emit Rust that rustc will refuse, and that is exactly the
    // class of bug worth hunting.
    let show = files.iter().any(|argument| argument == "--rust");
    files.retain(|argument| argument != "--rust");
    if files.is_empty() {
        eprintln!("usage: judge <fichier.rsx> [fichier.rsx …]");
        std::process::exit(2);
    }

    let mut refused = 0;
    for file in &files {
        let path = Path::new(file);
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(oops) => {
                println!("{file}: unreadable ({oops})");
                refused += 1;
                continue;
            }
        };

        match rustx_compiler::transpile::transpile(&source) {
            Ok(done) => {
                let emitted = done.rust.to_string();
                println!(
                    "{file}: accepted — {} lines in, {} chars of Rust out",
                    source.lines().count(),
                    emitted.len()
                );
                if show {
                    println!("{emitted}");
                }
            }
            Err(refusal) => {
                refused += 1;
                let said = refusal.in_file(path).to_string();
                println!("{file}: REFUSED");
                println!("  {said}");
                if let Some((line, column)) = coordinates(&said) {
                    if let Some(text) = source.lines().nth(line.saturating_sub(1)) {
                        println!("  {line:>4} | {text}");
                        println!("       | {}^", " ".repeat(column.saturating_sub(1)));
                    }
                }
            }
        }
    }

    if refused > 0 {
        std::process::exit(1);
    }
}

/// `…/page.rsx:72:67: unclosed `{`` → `(72, 67)`.
fn coordinates(said: &str) -> Option<(usize, usize)> {
    let mut numbers = said
        .split(':')
        .filter_map(|piece| piece.trim().parse::<usize>().ok());
    Some((numbers.next()?, numbers.next()?))
}
