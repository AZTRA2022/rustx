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

/// Un dossier que ce test est seul a avoir.
///
/// Le compteur compte : l'horloge de macOS ne descend pas sous la
/// microseconde, deux tests demarres ensemble recevaient le meme nom, et
/// le `remove_dir_all` de l'un emportait les pages de l'autre.
fn a_directory_of_its_own(what: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let mine = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("rustx-{what}-{}-{mine}", std::process::id()))
}

/// An application whose pages have been generated once.
fn an_application() -> std::path::PathBuf {
    let root = a_directory_of_its_own("reading");
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
    let workspace = a_directory_of_its_own("workspace");
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

#[test]
fn the_vocabulary_is_the_macros() {
    // Every name `#[rustx::main]` gives a page must be given to the
    // reading crate too, or the page's types are unknown there and nothing
    // completes.
    let macros = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../rustx-macros/src/lib.rs"),
    )
    .expect("the macros' source");
    for line in crate_for_pages::VOCABULARY.lines() {
        let wanted = line.replace("pub use", "pub(crate) use");
        assert!(
            macros.contains(&wanted),
            "the macro does not say `{wanted}`"
        );
    }
}

#[test]
fn pages_are_modules_of_the_reading_crate_and_not_included() {
    let root = an_application();
    let pages = rustx_compiler::build::generated_directory(&root);
    fs::create_dir_all(&pages).expect("a pages directory");
    fs::write(
        pages.join(rustx_compiler::build::PAGES),
        "mod page_0 { include!(\"page.rs\"); }\n",
    )
    .expect("the pages");
    let manifest = crate_for_pages::written_for(&root).expect("a reading crate");
    let written = fs::read_to_string(manifest.with_file_name("editor.rs")).expect("source");
    assert!(written.contains("#[path = \"page.rs\"]"), "got {written}");
    assert!(!written.contains("include!"), "got {written}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_entry_files_imports_and_modules_are_in_the_pages_scope() {
    let root = an_application();
    fs::create_dir_all(root.join("src/shop")).expect("src");
    fs::write(
        root.join("src/main.rs"),
        "pub mod shop;\nuse rustx::{\n    App,\n    Config,\n};\n\n#[rustx::main]\nfn main() {}\n",
    )
    .expect("entry");
    fs::write(root.join("src/shop.rs"), "pub mod cart;\n").expect("module");
    let pages = rustx_compiler::build::generated_directory(&root);
    fs::create_dir_all(&pages).expect("pages");
    fs::write(
        pages.join(rustx_compiler::build::PAGES),
        "mod page_0 { include!(\"page.rs\"); }\n",
    )
    .expect("pages.rs");
    fs::write(pages.join("page.rs"), "fn Page() {}\n").expect("a page the editor never opened");
    let manifest = crate_for_pages::written_for(&root).expect("a reading crate");
    let written = fs::read_to_string(manifest.with_file_name("editor.rs")).expect("source");
    // Under an inline module over `src/`, so `shop/cart.rs` is found
    // where rustc finds it.
    assert!(written.contains("#[path = \"src\"]"), "got {written}");
    assert!(written.contains("pub mod shop;"), "got {written}");
    // A multi-line `use` stays whole.
    assert!(
        written.contains("pub use rustx::{\n        App,\n        Config,\n    };"),
        "got {written}"
    );
    // And a page nobody opened has its scope.
    let page = fs::read_to_string(pages.join("page.rs")).expect("page");
    assert!(page.ends_with(crate_for_pages::SCOPE), "got {page}");
    let _ = fs::remove_dir_all(&root);
}
