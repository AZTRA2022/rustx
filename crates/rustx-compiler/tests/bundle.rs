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

/// A `node_modules` holding one package at one version.
fn installed(app: &std::path::Path, package: &str, version: &str) {
    let directory = package
        .split('/')
        .fold(app.join("node_modules"), |at, part| at.join(part));
    std::fs::create_dir_all(&directory).expect("a package directory");
    std::fs::write(
        directory.join("package.json"),
        format!("{{\n  \"name\": \"{package}\",\n  \"version\": \"{version}\"\n}}\n"),
    )
    .expect("a package manifest");
}

#[test]
fn the_tailwind_a_project_installed_is_the_tailwind_it_is_driven_as() {
    use rustx_compiler::bundle::Dialect;

    let three = somewhere("tailwind-3");
    installed(&three, "tailwindcss", "3.4.17");
    assert_eq!(rustx_compiler::bundle::dialect_in(&three), Dialect::Content);

    let four = somewhere("tailwind-4");
    installed(&four, "tailwindcss", "4.1.14");
    assert_eq!(rustx_compiler::bundle::dialect_in(&four), Dialect::Source);

    let _ = std::fs::remove_dir_all(&three);
    let _ = std::fs::remove_dir_all(&four);
}

#[test]
fn a_version_nobody_can_read_goes_by_the_cli_that_is_installed() {
    use rustx_compiler::bundle::Dialect;

    // Tailwind 4 moved its command line into a package of its own, so a
    // project that has `@tailwindcss/cli` is a project on 4 whatever
    // else can or cannot be read.
    let app = somewhere("tailwind-unreadable");
    installed(&app, "@tailwindcss/cli", "4.1.14");
    assert_eq!(rustx_compiler::bundle::dialect_in(&app), Dialect::Source);

    let nothing = somewhere("tailwind-nothing");
    assert_eq!(
        rustx_compiler::bundle::dialect_in(&nothing),
        Dialect::Content
    );

    let _ = std::fs::remove_dir_all(&app);
    let _ = std::fs::remove_dir_all(&nothing);
}

#[test]
fn tailwind_4_is_told_its_sources_in_the_stylesheet_and_not_on_the_command_line() {
    use rustx_compiler::bundle::{arguments_for, input_for, Dialect};

    // `--content` does not exist in 4: the sources are `@source` lines
    // in the stylesheet, and passing the flag is the error a developer
    // sees instead of a page.
    let arguments = arguments_for(Dialect::Source, std::path::Path::new("in.css"), std::path::Path::new("out.css"));
    assert!(
        !arguments.iter().any(|argument| argument == "--content"),
        "got {arguments:?}"
    );
    let arguments = arguments_for(Dialect::Content, std::path::Path::new("in.css"), std::path::Path::new("out.css"));
    assert!(
        arguments.iter().any(|argument| argument == "--content"),
        "3 has no other way to be told: {arguments:?}"
    );

    let generated = input_for(Dialect::Source, false).expect("a generated stylesheet");
    assert!(generated.contains("@import \"tailwindcss\""), "got {generated}");
    assert!(
        generated.contains("@source \"../app/**/*.rsx\""),
        "the pages must be scanned: {generated}"
    );
    assert!(!generated.contains("@tailwind"), "3's syntax: {generated}");

    let generated = input_for(Dialect::Content, false).expect("a generated stylesheet");
    assert!(generated.contains("@tailwind utilities"), "got {generated}");
}

#[test]
fn an_applications_own_stylesheet_is_imported_and_not_replaced() {
    use rustx_compiler::bundle::{input_for, Dialect};

    // On 3 it is handed to the command line as it is. On 4 it has to be
    // imported, because the `@source` lines the framework adds have to
    // live in the same stylesheet — and a theme a developer wrote is
    // not something to drop on the way.
    assert_eq!(input_for(Dialect::Content, true), None);
    let generated = input_for(Dialect::Source, true).expect("a generated stylesheet");
    assert!(
        generated.contains("@import \"../assets/tailwind.css\""),
        "got {generated}"
    );
    assert!(
        !generated.contains("@import \"tailwindcss\""),
        "the application's own stylesheet imports it: {generated}"
    );
}
