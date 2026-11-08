//! What has to be true of **every** page in the repository.
//!
//! The other test files use a page written for them. This one uses the
//! real ones: every application's, and the laboratory's when it sits
//! beside the repository. The absence of this file is what let a
//! truncated line in `labo-rustx/app/page.rsx` survive a green report —
//! nothing read that page, so nothing could say so.
//!
//! A law here does not look at what a page means. It looks at what no
//! page can afford to be.

use std::path::{Path, PathBuf};

/// The repository, from the crate this test belongs to.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository is above the crate")
}

/// The laboratory, when the sibling checkout is there.
fn laboratory() -> Option<PathBuf> {
    let beside = repository().join("../labo-rustx");
    beside.canonicalize().ok().filter(|here| here.is_dir())
}

/// Every `.rsx` under `at`, however deep.
fn pages_under(at: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(at) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            // A build directory holds generated copies of these pages,
            // which would be judged twice and under a name nobody wrote.
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            pages_under(&path, into);
        } else if path.extension().is_some_and(|kind| kind == "rsx") {
            into.push(path);
        }
    }
}

/// Every page to be judged.
fn every_page() -> Vec<PathBuf> {
    let mut found = Vec::new();
    pages_under(&repository().join("apps"), &mut found);
    if let Some(laboratory) = laboratory() {
        pages_under(&laboratory.join("app"), &mut found);
    }
    found.sort();
    found
}

/// The short name a failure has to carry.
fn named(page: &Path) -> String {
    match page.strip_prefix(repository()) {
        Ok(under) => under.display().to_string(),
        // The laboratory is not under the repository: what identifies it
        // is its name, not the verbatim path `canonicalize` returns.
        Err(_) => {
            let tail: Vec<String> = page
                .components()
                .rev()
                .take(3)
                .map(|piece| piece.as_os_str().to_string_lossy().to_string())
                .collect();
            tail.into_iter().rev().collect::<Vec<_>>().join("/")
        }
    }
}

/// The emitted Rust, with paths spelled one way.
///
/// Two emission paths live side by side: one writes the text by hand
/// (`rustx_dom::Element`) and one goes through the token printer
/// (`rustx_dom :: Element`). A law that looks for a name has to look for
/// the name, not for one of its two spellings.
fn said_plainly(rust: &str) -> String {
    rust.replace(" :: ", "::")
}

/// Does this emitted Rust hold together as tokens?
///
/// A page is sometimes a whole file and sometimes a fragment — a page
/// whose body is one `return <main …>;` has no item in it at all.
/// Either shape is fine; tokens that close nothing are not.
fn holds_as_rust(rust: &str) -> Result<(), syn::Error> {
    syn::parse_file(rust).map(|_| ()).or_else(|_| {
        syn::parse_str::<syn::Block>(&(String::from("{") + rust + "}")).map(|_| ())
    })
}

#[test]
fn there_are_pages_to_judge() {
    // A net that goes quiet when it covers nothing is the defect this
    // file repairs. The count comes from the applications alone, so one
    // machine's laboratory cannot satisfy it.
    let mut from_applications = Vec::new();
    pages_under(&repository().join("apps"), &mut from_applications);
    assert!(
        from_applications.len() >= 8,
        "only {} pages were found under apps/: {:?}",
        from_applications.len(),
        from_applications
    );
}

#[test]
fn the_laboratory_is_covered_or_said_to_be_missing() {
    // The laboratory lives outside the repository. When it is there it
    // is judged like the rest; when it is not, this says so instead of
    // passing quietly on coverage it does not have.
    match laboratory() {
        Some(here) => {
            let mut found = Vec::new();
            pages_under(&here.join("app"), &mut found);
            assert!(
                !found.is_empty(),
                "the laboratory is at {} and has no page",
                here.display()
            );
        }
        None => {
            eprintln!("note: ../labo-rustx is not beside the repository; its pages are not judged")
        }
    }
}

#[test]
fn every_page_of_the_repository_is_accepted() {
    // The repository's own pages only. The laboratory is a sibling
    // checkout somebody is writing in: a half-typed `let` there is a
    // keystroke, not a defect, and a net that cries wolf at one is a
    // net that gets ignored. What the laboratory is still held to is
    // the law below — when it does compile, it must not emit nonsense,
    // which is the law that catches a truncated line.
    let mut ours = Vec::new();
    pages_under(&repository().join("apps"), &mut ours);
    for page in ours {
        let source = std::fs::read_to_string(&page).expect("a page reads");
        if let Err(refused) = rustx_compiler::transpile::transpile(&source) {
            panic!("{} is refused: {}", named(&page), refused.message);
        }
    }
}

#[test]
fn what_every_page_emits_is_rust_that_holds() {
    // A page names what its application defines, so `rustc` cannot see
    // one from here. But half of what `rustc` would say is about the
    // shape of the tokens — a brace that closes nothing, a lost
    // parenthesis — and that half is exactly what a transpiler gets
    // wrong.
    for page in every_page() {
        let source = std::fs::read_to_string(&page).expect("a page reads");
        let Ok(built) = rustx_compiler::transpile::transpile(&source) else {
            continue;
        };
        if let Err(broken) = holds_as_rust(&built.rust) {
            eprintln!("{} emitted:", named(&page));
            eprintln!("{}", built.rust);
            panic!("{} does not emit Rust that holds: {broken}", named(&page));
        }
    }
}

#[test]
fn a_truncated_page_does_not_pass_the_law_that_would_catch_it() {
    // The law above is only worth what it refuses. This feeds it the
    // defect it exists for: a real page of the repository with the tail
    // of one line cut off, which is what happened to the laboratory's
    // page and went unnoticed because nothing read it.
    let mut cut = 0;
    for page in every_page() {
        let source = std::fs::read_to_string(&page).expect("a page reads");
        let Some(at) = source.find(");\n") else {
            continue;
        };
        let wounded = String::from(&source[..at]) + "\n" + &source[at + ");\n".len()..];
        let Ok(built) = rustx_compiler::transpile::transpile(&wounded) else {
            // A refusal is the other way to catch it, and `transpile`
            // refusing is the law above it doing its work.
            cut += 1;
            continue;
        };
        assert!(
            holds_as_rust(&built.rust).is_err(),
            "{}, with a line truncated, still emits Rust that holds",
            named(&page)
        );
        cut += 1;
    }
    assert!(cut > 0, "no page offered a line to truncate");
}

#[test]
fn every_page_keeps_its_line_count() {
    // Plan 2's position map rests on line numbers: a rewrite that loses
    // one moves every diagnostic below it, without breaking anything
    // visibly.
    for page in every_page() {
        let source = std::fs::read_to_string(&page).expect("a page reads");
        let Ok(built) = rustx_compiler::transpile::transpile(&source) else {
            continue;
        };
        assert_eq!(
            built.rust.matches('\n').count(),
            source.matches('\n').count(),
            "{} changed its line count",
            named(&page)
        );
    }
}

#[test]
fn no_page_emits_nothing() {
    // An edit whose span slipped can swallow the file. The result then
    // compiles perfectly, and the window is empty.
    for page in every_page() {
        let source = std::fs::read_to_string(&page).expect("a page reads");
        let Ok(built) = rustx_compiler::transpile::transpile(&source) else {
            continue;
        };
        assert!(
            said_plainly(&built.rust).contains("rustx_dom::Element"),
            "{} emits no element",
            named(&page)
        );
    }
}

#[test]
fn an_expression_that_holds_statements_is_emitted_as_a_block() {
    // `{ let (visites, rang) = …; format!(…) }` dans une balise : les
    // accolades de la balise étaient prises pour celles de
    // l'expression, donc ce qui sortait était une suite d'instructions
    // là où un argument était attendu — `into_element(let … ; …)`, que
    // rien ne peut analyser.
    let source = "fn Page() -> Element {\n    \
                  return <p>{ let combien = 2; format!(\"{combien}\") }</p>;\n}\n";
    let built = rustx_compiler::transpile::transpile(source).expect("transpilable");
    holds_as_rust(&built.rust).unwrap_or_else(|broken| {
        panic!("emitted:\n{}\n\n{broken}", built.rust);
    });
}

#[test]
fn a_name_a_let_binds_is_not_a_read_of_the_page_value_of_that_name() {
    // La page déclare `rang`, et une expression de balise lie un `rang`
    // à elle. Le nom à gauche d'un `let` est une déclaration, pas une
    // lecture : réécrit, il donnait `let (rang.get(), total) = …`.
    let source = "let mut rang = 0_usize;\n\n\
                  fn Page() -> Element {\n    \
                  return <p>{ let (rang, total) = (1, 2); format!(\"{rang}/{total}\") }</p>;\n}\n";
    let built = rustx_compiler::transpile::transpile(source).expect("transpilable");
    assert!(
        !built.rust.contains("rang . get () , total"),
        "le motif du `let` a été réécrit :\n{}",
        built.rust
    );
    holds_as_rust(&built.rust).unwrap_or_else(|broken| {
        panic!("emitted:\n{}\n\n{broken}", built.rust);
    });
}
