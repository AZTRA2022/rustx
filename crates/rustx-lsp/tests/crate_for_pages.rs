//! The crate whose source is the pages.
//!
//! Its shape is not a matter of taste: `rust-analyzer` loads a crate's
//! own directory and never reads an `include!` that points outside it.
//! A crate written beside the pages answered nothing at all — proved by
//! hand against a real `rust-analyzer`, patiently, for four minutes. A
//! crate rooted *at* the pages answers in seconds. So the test that
//! matters is where the manifest lands.

use std::fs;

use rustx_lsp::crate_for_pages;

/// An application whose pages have been generated once.
fn an_application() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "rustx-reading-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(root.join("app")).expect("an app directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"read\"\n\n[dependencies]\nrustx = { path = \"../../crates/rustx\" }\nserde = \"1\"\n",
    )
    .expect("a manifest");
    root
}

fn pages_written(root: &std::path::Path) {
    let pages = rustx_compiler::build::generated_directory(root);
    fs::create_dir_all(&pages).expect("a pages directory");
    fs::write(
        pages.join(rustx_compiler::build::PAGES),
        "fn Page() -> Element { rustx_dom::Element::Empty }\n",
    )
    .expect("the pages");
}

#[test]
fn the_crate_is_rooted_at_the_pages_themselves() {
    let root = an_application();
    pages_written(&root);
    let manifest = crate_for_pages::written_for(&root).expect("a reading crate");
    assert_eq!(
        manifest.parent(),
        Some(rustx_compiler::build::generated_directory(&root).as_path()),
        "a crate anywhere else cannot read the pages"
    );
    let written = fs::read_to_string(&manifest).expect("on disk");
    // Its own workspace, or cargo reads it as a member of the one it
    // sits inside and refuses.
    assert!(written.contains("[workspace]"), "got {written}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_pages_are_included_by_a_source_file_of_that_crate() {
    let root = an_application();
    pages_written(&root);
    let manifest = crate_for_pages::written_for(&root).expect("a reading crate");
    let library = manifest.with_file_name("editor.rs");
    let written = fs::read_to_string(&library).expect("the source is written");
    assert!(written.contains("include!(\"pages.rs\")"), "got {written}");
    // The vocabulary a page is compiled with, as the attribute macro
    // gives it.
    assert!(written.contains("rustx::Element"), "got {written}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_dependencies_are_the_applications_own_made_absolute() {
    // A page sees what the application sees. A relative path would
    // point at nothing from under `target/`, and a dependency that does
    // not resolve is a page with no vocabulary at all.
    let root = an_application();
    pages_written(&root);
    let manifest = crate_for_pages::written_for(&root).expect("a reading crate");
    let written = fs::read_to_string(&manifest).expect("on disk");
    assert!(written.contains("serde = \"1\""), "got {written}");
    assert!(written.contains("rustx = { path = \""), "got {written}");
    assert!(
        !written.contains("\"../../crates/rustx\""),
        "a path that points at nothing: {written}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_application_that_was_never_built_has_nothing_to_read() {
    let root = an_application();
    assert!(crate_for_pages::written_for(&root).is_none());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn every_application_of_a_workspace_is_found() {
    let workspace = std::env::temp_dir().join(format!(
        "rustx-workspace-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    for name in ["one", "two"] {
        let app = workspace.join("apps").join(name);
        fs::create_dir_all(app.join("app")).expect("an app directory");
        fs::write(app.join("Cargo.toml"), "[package]\nname = \"x\"\n").expect("a manifest");
        pages_written(&app);
    }
    let found = crate_for_pages::for_workspace(&workspace);
    assert_eq!(found.len(), 2, "got {found:?}");
    let _ = fs::remove_dir_all(&workspace);
}
