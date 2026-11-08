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

// --- What the hunt of 2026-11-06 found, in an application built on the
// --- framework from the outside: Tailwind 4, TipTap, shadcn.

#[test]
fn a_scoped_build_tool_is_not_a_library_of_the_page() {
    // `npm install tailwindcss @tailwindcss/cli` is the first line of
    // Tailwind 4's own getting-started page, and it puts the command
    // line under `dependencies`. The bundle used to import it, and
    // esbuild refused the whole build.
    let entry = entry_for(&["@tailwindcss/cli".to_string(), "recharts".to_string()]);
    assert!(!entry.contains("tailwindcssCli"), "got {entry}");
    assert!(entry.contains("import * as recharts"), "got {entry}");
}

#[test]
fn a_package_with_sub_paths_and_no_root_is_left_out_of_the_entry() {
    // `@tiptap/pm` exports `./state`, `./view`, and no `.`. A page
    // writes `@tiptap/pm/state` or nothing, and importing the root
    // used to be an esbuild error about a path nobody wrote.
    let app = somewhere("sub-paths-only");
    write_package(
        &app,
        "{\"dependencies\": {\"@tiptap/pm\": \"3.0.0\", \"@tiptap/react\": \"3.0.0\"}}",
    );
    install(
        &app,
        "@tiptap/pm",
        "{\"name\": \"@tiptap/pm\", \"exports\": {\"./state\": \"./state/index.js\"}}",
    );
    install(
        &app,
        "@tiptap/react",
        "{\"name\": \"@tiptap/react\", \"main\": \"dist/index.js\"}",
    );
    assert_eq!(
        rustx_compiler::bundle::installed(&app, "@tiptap/pm"),
        rustx_compiler::bundle::Installed::NothingToImport
    );
    assert_eq!(
        rustx_compiler::bundle::installed(&app, "@tiptap/react"),
        rustx_compiler::bundle::Installed::Importable
    );
}

#[test]
fn a_package_whose_entry_point_is_a_stylesheet_is_left_out_too() {
    // `tw-animate-css` belongs to the stylesheet, not to the bundle.
    let app = somewhere("css-only");
    install(
        &app,
        "tw-animate-css",
        "{\"name\": \"tw-animate-css\", \"main\": \"dist/tw-animate.css\"}",
    );
    assert_eq!(
        rustx_compiler::bundle::installed(&app, "tw-animate-css"),
        rustx_compiler::bundle::Installed::NothingToImport
    );
}

#[test]
fn a_library_declared_and_never_installed_is_said_by_name() {
    let app = somewhere("declared-not-installed");
    write_package(&app, "{\"dependencies\": {\"recharts\": \"2.12.7\"}}");
    // An esbuild to get past the first refusal, and nothing else.
    install(&app, "esbuild", "{\"name\": \"esbuild\"}");
    let binaries = app.join("node_modules").join(".bin");
    std::fs::create_dir_all(&binaries).expect("a .bin");
    std::fs::write(binaries.join("esbuild.cmd"), "").expect("a file");

    let said = build(&app).expect_err("should refuse");
    assert!(said.contains("recharts"), "got {said}");
    assert!(said.contains("npm install"), "got {said}");
}

#[test]
fn a_components_file_outside_ui_is_a_tag_as_well() {
    // `components/ui` is where `shadcn add` leaves its own;
    // `components/` is where a developer leaves theirs, and the
    // Tailwind globs beside this have always read both. The index read
    // only the first, so `<Editor>` compiled and found nothing.
    let app = somewhere("own-components-nested");
    let components = app.join("components");
    std::fs::create_dir_all(components.join("ui")).expect("a directory");
    std::fs::write(
        components.join("Editor.tsx"),
        "export default function Editor() { return null }\n",
    )
    .expect("a file");
    std::fs::write(
        components.join("ui").join("button.tsx"),
        "export function Button() { return null }\n",
    )
    .expect("a file");

    let index = rustx_compiler::bundle::index_for(&app).expect("an index");
    assert!(index.contains("../components/ui/button.tsx"), "got {index}");
    assert!(index.contains("../components/Editor.tsx"), "got {index}");
    // And the default export, under the name of its file, because
    // `export *` does not carry a default.
    assert!(
        index.contains("export { default as Editor }"),
        "got {index}"
    );
}

#[test]
fn tailwind_installed_with_nothing_to_run_it_is_said_and_not_skipped() {
    // Tailwind 4 keeps its command line in `@tailwindcss/cli`. An
    // application that installed only the library used to get a window
    // with every `className` in it and not one rule, and no word about
    // it anywhere.
    let app = somewhere("tailwind-without-a-cli");
    write_package(&app, "{\"dependencies\": {\"tailwindcss\": \"^4.0.0\"}}");
    install(&app, "tailwindcss", "{\"name\": \"tailwindcss\", \"version\": \"4.1.0\"}");

    let said = build(&app).expect_err("should refuse");
    assert!(said.contains("@tailwindcss/cli"), "got {said}");
}

/// A `package.json` for the application itself.
fn write_package(app: &std::path::Path, text: &str) {
    std::fs::write(app.join("package.json"), text).expect("a manifest");
}

/// A package in `node_modules`, with the manifest it would have.
fn install(app: &std::path::Path, package: &str, manifest: &str) {
    let directory = package
        .split('/')
        .fold(app.join("node_modules"), |at, part| at.join(part));
    std::fs::create_dir_all(&directory).expect("a package directory");
    std::fs::write(directory.join("package.json"), manifest).expect("a manifest");
}

#[test]
fn two_libraries_that_come_to_one_name_are_refused() {
    // `@tiptap/react` and `tiptap-react` both become `tiptapReact`. The
    // entry point would write one over the other in a single object
    // literal, which JavaScript allows and nobody can see.
    let collision = rustx_compiler::bundle::a_collision_in(&[
        "@tiptap/react".to_string(),
        "recharts".to_string(),
        "tiptap-react".to_string(),
    ])
    .expect("a collision");
    assert_eq!(
        collision,
        ("@tiptap/react".to_string(), "tiptap-react".to_string())
    );
    assert!(
        rustx_compiler::bundle::a_collision_in(&[
            "@tiptap/react".to_string(),
            "recharts".to_string()
        ])
        .is_none()
    );
}
