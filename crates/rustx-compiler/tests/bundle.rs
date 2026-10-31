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
    assert_eq!(build(&empty), Ok(Default::default()));
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

#[test]
fn an_applications_own_components_are_a_library_too() {
    // What `npx shadcn add button` leaves behind: TypeScript components
    // in the project itself, not in `node_modules`. They are the
    // application's, so nothing installs them and nothing imports them
    // — which is exactly why the framework has to, or a page could
    // write `<Button />` and find nothing.
    let into = somewhere("own-components");
    let ui = into.join("components").join("ui");
    std::fs::create_dir_all(&ui).expect("a components directory");
    std::fs::write(
        ui.join("button.tsx"),
        "export const Button = (props) => null;\nexport const buttonVariants = () => \"\";\n",
    )
    .expect("a component");
    std::fs::write(
        ui.join("card.tsx"),
        "export const Card = (props) => null;\n",
    )
    .expect("a card");

    let index = rustx_compiler::bundle::index_for(&into).expect("an index for the components");
    assert!(
        index.contains("export * from \"../components/ui/button.tsx\""),
        "every component file is re-exported: {index}"
    );
    assert!(
        index.contains("card.tsx"),
        "a component added later is in too: {index}"
    );

    let entry = entry_for_project(&into);
    assert!(
        entry.contains("import * as ui from \"./ui.ts\""),
        "the application's own components are imported: {entry}"
    );
    assert!(entry.contains("ui,"), "and handed to the runtime: {entry}");
    let _ = std::fs::remove_dir_all(&into);
}

#[test]
fn an_application_without_its_own_components_imports_nothing_extra() {
    let into = somewhere("no-components");
    std::fs::create_dir_all(&into).expect("a project");
    assert_eq!(rustx_compiler::bundle::index_for(&into), None);
    let entry = entry_for_project(&into);
    assert!(!entry.contains("./ui.ts"), "got {entry}");
    let _ = std::fs::remove_dir_all(&into);
}

#[test]
fn tailwind_reads_the_applications_own_components() {
    // A class written in a `.tsx` of the project — which is where every
    // shadcn component keeps its classes — must reach the stylesheet,
    // or the components arrive unstyled and nothing says why.
    let scanned = rustx_compiler::bundle::content_globs();
    assert!(scanned.contains("./app/**/*.rsx"), "got {scanned}");
    assert!(scanned.contains("./src/**/*.rs"), "got {scanned}");
    assert!(
        scanned.contains("./components/**/*.{ts,tsx,js,jsx}"),
        "got {scanned}"
    );
    assert!(
        scanned.contains("./lib/**/*.{ts,tsx,js,jsx}"),
        "a shadcn project keeps `cn` and its variants in `lib/`: {scanned}"
    );
}

fn entry_for_project(project: &std::path::Path) -> String {
    rustx_compiler::bundle::entry_for_in(&["react".to_string()], project)
}

fn somewhere(what: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rustx-bundle-{what}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&path).expect("a directory");
    path
}
