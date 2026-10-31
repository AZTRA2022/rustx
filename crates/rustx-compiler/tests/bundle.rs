//! The JavaScript the framework writes for an application.
//!
//! An application declares its libraries in `package.json` and writes
//! its components in `.rsx`. The entry point that imports those
//! libraries and hands them to the runtime is nobody's to write.

use rustx_compiler::bundle::{build, entry_for};

#[test]
fn an_entry_imports_every_dependency_under_a_usable_name() {
    let entry = entry_for(&["recharts".to_string()]);
    assert!(
        entry.contains("import * as recharts from \"recharts\""),
        "got {entry}"
    );
    // React itself, which every boundary needs and no page names.
    assert!(entry.contains("from \"react\""), "got {entry}");
    assert!(entry.contains("from \"react-dom\""), "got {entry}");
    assert!(entry.contains("from \"react-dom/client\""), "got {entry}");
    assert!(entry.contains("window.rustxLibraries"), "got {entry}");
    assert!(
        entry.contains("recharts,") || entry.contains("recharts }"),
        "got {entry}"
    );
}

#[test]
fn a_scoped_package_gets_a_camel_case_name() {
    // `@radix-ui/react-dialog` is not an identifier. What the page
    // writes is `<radixUiReactDialog::Root />`, or just `<Root />`.
    let entry = entry_for(&["@radix-ui/react-dialog".to_string()]);
    assert!(
        entry.contains("import * as radixUiReactDialog from \"@radix-ui/react-dialog\""),
        "got {entry}"
    );
}

#[test]
fn the_build_tools_themselves_are_not_libraries_of_the_page() {
    let entry = entry_for(&["esbuild".to_string(), "tailwindcss".to_string()]);
    assert!(!entry.contains("import * as esbuild"), "got {entry}");
    assert!(!entry.contains("import * as tailwindcss"), "got {entry}");
}

#[test]
fn an_application_with_no_package_json_generates_nothing() {
    let empty = std::env::temp_dir().join(format!("rustx-bundle-empty-{}", std::process::id()));
    std::fs::create_dir_all(&empty).expect("a directory");
    assert_eq!(build(&empty), Ok(None));
}

#[test]
fn a_missing_node_modules_says_to_install() {
    let app = std::env::temp_dir().join(format!("rustx-bundle-bare-{}", std::process::id()));
    std::fs::create_dir_all(&app).expect("a directory");
    std::fs::write(
        app.join("package.json"),
        "{\"dependencies\": {\"recharts\": \"2.12.7\"}}",
    )
    .expect("a file");
    let said = build(&app).expect_err("should refuse");
    assert!(said.contains("npm install"), "got {said}");
}
