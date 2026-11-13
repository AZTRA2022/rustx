//! The document the screens are put in.
//!
//! An application that has written no HTML is an application that wants
//! the obvious one: a page with a host element and nothing else. Writing
//! it by hand is ceremony, and ceremony is what this crate exists to
//! take over.

use std::path::Path;

use rustx::document_for;

fn tree(files: &[(&str, &str)]) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("rustx-document-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, contents) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        std::fs::write(&path, contents).expect("a file");
    }
    std::fs::create_dir_all(&root).expect("a directory");
    root
}

#[test]
fn the_applications_own_document_is_used_when_there_is_one() {
    let root = tree(&[("assets/index.html", "<!doctype html><p>mine</p>")]);
    let document = document_for(&root, "Console").expect("a document");
    assert_eq!(document, root.join("assets").join("index.html"));
}

#[test]
fn an_application_with_no_document_is_given_one() {
    let root = tree(&[]);
    let document = document_for(&root, "Console").expect("a document");
    assert!(document.exists(), "no document at {}", document.display());

    let text = std::fs::read_to_string(&document).expect("readable");
    // The host the router mounts under, and the title the application
    // gave the window — the two things the document is for.
    assert!(text.contains("id=\"app\""), "got {text}");
    assert!(text.contains("<title>Console</title>"), "got {text}");
}

#[test]
fn a_generated_document_resolves_relative_paths_against_the_applications_assets() {
    // The generated document is not written beside the assets — it goes
    // under the temporary files, where nothing else is. Without a
    // `<base>` the one relative URL the framework itself writes,
    // `rustx-bundle.js`, is looked for in that empty directory, the
    // page gets none of its libraries, and every React component in it
    // reports that nothing has it.
    let root = tree(&[("assets/rustx-bundle.js", "window.rustxLibraries = {};")]);
    let document = document_for(&root, "Console").expect("a document");
    assert_ne!(
        document.parent(),
        root.join("assets").parent(),
        "this test is only about the document that is generated elsewhere"
    );

    let text = std::fs::read_to_string(&document).expect("readable");
    let at = text.find("<base href=\"").expect(&format!("no base in {text}"));
    let href = &text[at + "<base href=\"".len()..];
    let href = &href[..href.find('"').expect("a closed attribute")];
    assert!(href.starts_with("file:///"), "got {href}");
    assert!(href.ends_with('/'), "a directory, not a file: {href}");

    // And it names the assets directory that exists, not one beside it.
    let path = href
        .trim_start_matches("file:///")
        .replace("%20", " ")
        .replace('/', std::path::MAIN_SEPARATOR_STR);
    let mut path = std::path::PathBuf::from(&path);
    if !cfg!(windows) {
        path = std::path::PathBuf::from(std::path::MAIN_SEPARATOR_STR).join(path);
    }
    assert!(
        path.join("rustx-bundle.js").exists(),
        "{} does not hold the bundle",
        path.display()
    );
}

#[test]
fn the_document_given_to_an_application_is_its_own() {
    // Two applications running at once must not be handed the same file:
    // the second one's title would replace the first one's.
    let first = document_for(&tree(&[]), "First").expect("a document");
    let second = document_for(&tree(&[]), "Second").expect("a document");
    assert_ne!(first, second);
    assert!(
        std::fs::read_to_string(&first)
            .expect("readable")
            .contains("First"),
        "the first application's document was overwritten"
    );
    assert!(Path::new(&second).exists());
}
