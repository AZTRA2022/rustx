//! The route table read from a directory of pages.

use std::path::{Path, PathBuf};

use rustx_compiler::routes::table_for;

const PAGE: &str = "fn Page() -> Element {\n    return <p>page</p>;\n}\n";
const PAGE_WITH_ID: &str = "fn Page(id: String) -> Element {\n    return <p>{id}</p>;\n}\n";
const LAYOUT: &str =
    "fn Layout(children: Element) -> Element {\n    return <main>{children}</main>;\n}\n";

/// A throwaway `app/` directory holding these files.
fn tree(files: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "rustx-routes-{}-{}-{id}",
        std::process::id(),
        std::thread::current().name().unwrap_or("x").replace(':', "_")
    ));
    let _ = std::fs::remove_dir_all(&root);

    for (path, contents) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        std::fs::write(&path, contents).expect("a file");
    }
    root
}

fn table(files: &[(&str, &str)]) -> String {
    table_for(Path::new(&tree(files))).expect("a table")
}

#[test]
fn a_page_at_the_root_is_the_root_route() {
    assert!(table(&[("page.rsx", PAGE)]).contains("pattern: \"/\""));
}

#[test]
fn a_folder_is_a_segment_and_a_bracketed_folder_is_a_parameter() {
    let generated = table(&[("device/[id]/page.rsx", PAGE_WITH_ID)]);
    assert!(
        generated.contains("pattern: \"/device/[id]\""),
        "got {generated}"
    );
    assert!(generated.contains("params.get(\"id\")"), "got {generated}");
}

#[test]
fn a_layout_encloses_every_page_below_it_outermost_first() {
    let generated = table(&[
        ("layout.rsx", LAYOUT),
        ("device/layout.rsx", LAYOUT),
        ("device/page.rsx", PAGE),
    ]);
    assert!(
        generated.contains("layouts: &[layout_0, layout_1]"),
        "got {generated}"
    );
}

#[test]
fn a_page_above_a_layout_is_not_wrapped_in_it() {
    let generated = table(&[
        ("page.rsx", PAGE),
        ("device/layout.rsx", LAYOUT),
        ("device/page.rsx", PAGE),
    ]);
    let root = generated
        .split("rustx_dom::route::Route {")
        .find(|route| route.contains("pattern: \"/\","))
        .unwrap_or_else(|| panic!("no root route in {generated}"));
    assert!(root.contains("layouts: &[],"), "got {root}");

    let device = generated
        .split("rustx_dom::route::Route {")
        .find(|route| route.contains("pattern: \"/device\","))
        .unwrap_or_else(|| panic!("no device route in {generated}"));
    assert!(device.contains("layouts: &[layout_0],"), "got {device}");
}

#[test]
fn each_page_is_included_in_a_module_of_its_own() {
    // Two pages both declaring `fn Page` must not collide.
    let generated = table(&[("page.rsx", PAGE), ("device/page.rsx", PAGE)]);
    assert_eq!(generated.matches("include!").count(), 2, "got {generated}");
}

#[test]
fn a_module_sees_what_the_application_has_in_scope() {
    // A page says `Element`, and the application's `main.rs` is what
    // brought that name in.
    let generated = table(&[("page.rsx", PAGE)]);
    assert!(generated.contains("use super::*;"), "got {generated}");
}

#[test]
fn the_table_is_the_same_whatever_order_the_files_come_in() {
    // §7.3: the transpilation is deterministic, and so is this.
    let one = table(&[("page.rsx", PAGE), ("device/page.rsx", PAGE)]);
    let two = table(&[("device/page.rsx", PAGE), ("page.rsx", PAGE)]);
    assert_eq!(one, two);
}

#[test]
fn a_page_without_a_page_component_is_refused_by_name() {
    let error = table_for(Path::new(&tree(&[(
        "page.rsx",
        "fn Other() -> Element { return <p>x</p>; }",
    )])))
    .expect_err("should be refused");
    assert!(error.message.contains("fn Page"), "got {}", error.message);
}

#[test]
fn a_layout_without_a_layout_component_is_refused_by_name() {
    let error = table_for(Path::new(&tree(&[
        ("page.rsx", PAGE),
        ("layout.rsx", "fn Other() -> Element { return <p>x</p>; }"),
    ])))
    .expect_err("should be refused");
    assert!(error.message.contains("fn Layout"), "got {}", error.message);
}

#[test]
fn a_page_parameter_that_names_no_segment_is_refused() {
    // `fn Page(nope: String)` under `device/[id]` has nowhere to get
    // `nope` from, and `rustc` would blame the generated call.
    let error = table_for(Path::new(&tree(&[(
        "device/[id]/page.rsx",
        "fn Page(nope: String) -> Element {\n    return <p>{nope}</p>;\n}\n",
    )])))
    .expect_err("should be refused");
    assert!(error.message.contains("nope"), "got {}", error.message);
}

#[test]
fn a_directory_with_no_page_at_all_is_an_empty_table() {
    let generated = table(&[("layout.rsx", LAYOUT)]);
    assert!(generated.contains("route_table"), "got {generated}");
    assert!(!generated.contains("pattern:"), "got {generated}");
}
