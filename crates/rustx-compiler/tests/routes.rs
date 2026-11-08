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
        std::thread::current()
            .name()
            .unwrap_or("x")
            .replace(':', "_")
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

#[test]
fn the_outermost_layout_is_the_first_index_for_every_page_below_it() {
    // C1 of the plan 5 review. The index list was built from an
    // alphabetical sort, where `device/layout.rsx` sorts before
    // `layout.rsx`, so the list ran innermost-first and the root layout
    // changed index between `/` and `/device` — changing its key, which
    // releases and rebuilds the component holding the navigation bar.
    // Asserting on the substring `&[layout_0, layout_1]` could not see
    // it: it is true in either ordering. So this test names the file
    // each index includes.
    let generated = table(&[
        ("layout.rsx", LAYOUT),
        ("device/layout.rsx", LAYOUT),
        ("page.rsx", PAGE),
        ("device/page.rsx", PAGE),
    ]);

    let first = generated
        .split("mod layout_0_file {")
        .nth(1)
        .unwrap_or_else(|| panic!("no layout_0 in {generated}"));
    assert!(
        first.contains("include!(\"layout.rs\")"),
        "layout_0 should be the outermost layout, got {first}"
    );
    let second = generated
        .split("mod layout_1_file {")
        .nth(1)
        .unwrap_or_else(|| panic!("no layout_1 in {generated}"));
    assert!(
        second.contains("include!(\"device/layout.rs\")"),
        "layout_1 should be the inner layout, got {second}"
    );

    let root = route_named(&generated, "/");
    assert!(root.contains("layouts: &[layout_0],"), "got {root}");
    let device = route_named(&generated, "/device");
    assert!(
        device.contains("layouts: &[layout_0, layout_1],"),
        "the root layout keeps index 0 under a page it also encloses, got {device}"
    );
}

/// The `Route` block for one pattern.
fn route_named<'a>(generated: &'a str, pattern: &str) -> &'a str {
    generated
        .split("rustx_dom::route::Route {")
        .find(|route| route.contains(&format!("pattern: \"{pattern}\",")))
        .unwrap_or_else(|| panic!("no `{pattern}` route in {generated}"))
}

#[test]
fn a_layout_that_does_not_take_its_children_is_refused_by_name() {
    // I7 of the plan 5 review. The parameters of a `layout.rsx` were
    // read and discarded, so `fn Layout()` compiled into a call with
    // one argument and the developer met a rustc arity error inside
    // OUT_DIR — a file they never wrote and cannot find.
    let error = table_for(Path::new(&tree(&[
        (
            "layout.rsx",
            "fn Layout() -> Element { return <main>x</main>; }",
        ),
        ("page.rsx", PAGE),
    ])))
    .expect_err("should be refused");
    let message = error.message;
    assert!(
        message.contains("children"),
        "the complaint should name `children`, got {message}"
    );
}

#[test]
fn every_component_without_parameters_is_in_the_component_table() {
    // React mounts a rustx component by name. Which components an
    // application is willing to let it mount is not a decision worth
    // asking for: a component written in `app/` is a component.
    let generated = table(&[(
        "page.rsx",
        "fn Page() -> Element {\n    return <p>page</p>;\n}\n\
         fn Panel() -> Element {\n    return <p>panel</p>;\n}\n",
    )]);
    assert!(
        generated.contains("pub fn component_table()"),
        "got {generated}"
    );
    assert!(generated.contains("(\"Panel\""), "got {generated}");
    assert!(generated.contains("(\"Page\""), "got {generated}");
}

#[test]
fn a_component_with_parameters_is_not_in_the_component_table() {
    // React has nothing to fill them with, and a mount that could only
    // fail is worse than one that is not offered.
    let generated = table(&[(
        "page.rsx",
        "fn Page() -> Element {\n    return <p>page</p>;\n}\n\
         fn Row(label: String) -> Element {\n    return <li>{label}</li>;\n}\n",
    )]);
    assert!(!generated.contains("(\"Row\""), "got {generated}");
}

#[test]
fn a_shared_value_is_published_by_its_name() {
    // What a page declares at its top, a scenario can read by name —
    // which is what lets an application stop writing natives of its own
    // to look at its own state.
    let generated = table(&[(
        "page.rsx",
        "let mut points = 0;\nlet mut label = \"a\";\n\
         fn Page() -> Element {\n    return <p>{points}{label}</p>;\n}\n",
    )]);
    assert!(
        generated.contains("pub fn shared_values()"),
        "got {generated}"
    );
    assert!(generated.contains("(\"points\""), "got {generated}");
    assert!(generated.contains("(\"label\""), "got {generated}");
}

#[test]
fn a_shared_value_that_cannot_be_shown_is_not_published() {
    // A list is a value, not something a report line can carry. Naming
    // it would mean generating code that does not compile, for a test
    // nobody asked for.
    let generated = table(&[(
        "page.rsx",
        "let mut rows = Vec::new();\n\
         fn Page() -> Element {\n    return <p>{rows.len()}</p>;\n}\n",
    )]);
    assert!(!generated.contains("(\"rows\""), "got {generated}");
}

#[test]
fn a_value_a_layout_shares_is_in_the_table_like_a_pages_own() {
    // A bar above every screen counts the seconds, and the value it
    // counts is written at the top of the `layout.rsx`. Where the value
    // lives is the application's business, not the reader's: a scenario
    // asking for `ticks` by name should not have to know that it was
    // declared in a layout rather than in a page.
    let generated = table(&[
        ("page.rsx", PAGE),
        (
            "layout.rsx",
            "let mut ticks = 0;\n\n\
             fn Layout(children: Element) -> Element {\n    \
             return <main>uptime {ticks}{children}</main>;\n}\n",
        ),
    ]);
    assert!(
        generated.contains("(\"ticks\", (|| rustx_dom::store::shared(\"ticks\""),
        "got {generated}"
    );
    assert!(
        generated.contains("all.extend(layout_0_file::shared_values());"),
        "the table never asks the layout: {generated}"
    );
}

// ─────────────────────────── Groupes de routes ───────────────────────────

#[test]
fn a_folder_in_parentheses_is_not_a_segment() {
    // Le groupe de Next.js : `app/(boutique)/panier/page.rsx` est
    // `/panier`. Il existe pour qu'un layout puisse couvrir plusieurs
    // écrans sans ajouter un segment à l'URL de chacun.
    let generated = table(&[("(boutique)/panier/page.rsx", PAGE)]);
    assert!(
        generated.contains("pattern: \"/panier\""),
        "le groupe ne doit pas être dans l'URL : {generated}"
    );
}

#[test]
fn a_group_still_carries_its_layout() {
    let generated = table(&[
        ("(boutique)/layout.rsx", LAYOUT),
        ("(boutique)/panier/page.rsx", PAGE),
    ]);
    assert!(generated.contains("pattern: \"/panier\""), "{generated}");
    assert!(
        generated.contains("layouts: &[layout_0]"),
        "le layout du groupe doit entourer la page : {generated}"
    );
}

#[test]
fn a_group_at_the_root_leaves_the_root_the_root() {
    let generated = table(&[("(public)/page.rsx", PAGE)]);
    assert!(generated.contains("pattern: \"/\""), "{generated}");
}

// ──────────────────────────── Attrape-tout ────────────────────────────

const PAGE_WITH_CHEMIN: &str =
    "fn Page(chemin: String) -> Element {\n    return <p>{chemin}</p>;\n}\n";

#[test]
fn a_catch_all_folder_is_a_catch_all_segment() {
    let generated = table(&[("fichiers/[...chemin]/page.rsx", PAGE_WITH_CHEMIN)]);
    assert!(
        generated.contains("pattern: \"/fichiers/[...chemin]\""),
        "{generated}"
    );
    assert!(
        generated.contains("params.get(\"chemin\")"),
        "la page doit recevoir ce que le segment a pris : {generated}"
    );
}

#[test]
fn an_optional_catch_all_folder_is_one_too() {
    let generated = table(&[("fichiers/[[...chemin]]/page.rsx", PAGE_WITH_CHEMIN)]);
    assert!(
        generated.contains("pattern: \"/fichiers/[[...chemin]]\""),
        "{generated}"
    );
}

// ─────────────────── Un layout reçoit ses paramètres ───────────────────

#[test]
fn a_layout_can_take_the_parameters_of_its_own_folder() {
    // Le layout de `[proprietaire]/[nom]` est le cadre d'un dépôt
    // précis. Sans cela, il devait relire `router::current()` et
    // redécouper le chemin à la main.
    let layout = "fn Layout(children: Element, proprietaire: String) -> Element {\n    \
                  return <main>{proprietaire}{children}</main>;\n}\n";
    let generated = table(&[
        ("depots/[proprietaire]/layout.rsx", layout),
        ("depots/[proprietaire]/page.rsx", PAGE_WITH_PROPRIETAIRE),
    ]);
    assert!(
        generated.contains("Layout(children, params.get(\"proprietaire\").to_string())"),
        "le layout doit être appelé avec son paramètre : {generated}"
    );
}

const PAGE_WITH_PROPRIETAIRE: &str =
    "fn Page(proprietaire: String) -> Element {\n    return <p>{proprietaire}</p>;\n}\n";

#[test]
fn a_layout_asking_for_a_parameter_its_folder_has_not_got_is_refused() {
    let layout = "fn Layout(children: Element, inconnu: String) -> Element {\n    \
                  return <main>{inconnu}{children}</main>;\n}\n";
    let error = table_for(Path::new(&tree(&[
        ("depots/layout.rsx", layout),
        ("depots/page.rsx", PAGE),
    ])))
    .expect_err("this cannot be generated");
    assert!(
        error.message.contains("inconnu"),
        "le refus doit nommer le paramètre : {}",
        error.message
    );
}

#[test]
fn a_layout_whose_first_parameter_is_not_children_is_still_refused() {
    let layout = "fn Layout(quoi: Element) -> Element {\n    return <main>{quoi}</main>;\n}\n";
    let error = table_for(Path::new(&tree(&[
        ("layout.rsx", layout),
        ("page.rsx", PAGE),
    ])))
    .expect_err("this cannot be generated");
    assert!(error.message.contains("children"), "{}", error.message);
}

// ────────────────────── L'écran d'attente d'un segment ──────────────────────

const LOADING: &str =
    "fn Loading() -> Element {\n    return <p>un instant</p>;\n}\n";

#[test]
fn a_loading_file_becomes_the_screen_shown_while_a_page_has_nothing() {
    let generated = table(&[("depots/loading.rsx", LOADING), ("depots/page.rsx", PAGE)]);
    assert!(
        generated.contains("loading: Some(loading_0)"),
        "la route doit porter son écran d'attente : {generated}"
    );
}

#[test]
fn the_nearest_loading_file_is_the_one_a_page_gets() {
    let generated = table(&[
        ("loading.rsx", LOADING),
        ("depots/loading.rsx", LOADING),
        ("depots/[nom]/page.rsx", "fn Page(nom: String) -> Element {\n    return <p>{nom}</p>;\n}\n"),
    ]);
    // Deux fichiers d'attente, et c'est le plus proche de la page qui
    // la couvre -- comme un layout, mais un seul, le plus bas.
    assert!(
        generated.contains("loading: Some(loading_1)"),
        "le plus proche doit gagner : {generated}"
    );
}

#[test]
fn a_page_with_no_loading_file_above_it_has_none() {
    let generated = table(&[("page.rsx", PAGE)]);
    assert!(generated.contains("loading: None"), "{generated}");
}

// ──────────────────── La frontière d'erreur d'un segment ────────────────────

const ERREUR: &str =
    "fn Error(pourquoi: String) -> Element {\n    return <p>{pourquoi}</p>;\n}\n";

#[test]
fn an_error_file_becomes_the_screen_shown_when_a_page_falls_over() {
    let generated = table(&[("depots/error.rsx", ERREUR), ("depots/page.rsx", PAGE)]);
    assert!(
        generated.contains("failed: Some(error_0)"),
        "la route doit porter sa frontière d'erreur : {generated}"
    );
}

#[test]
fn the_nearest_error_file_is_the_one_a_page_gets() {
    let generated = table(&[
        ("error.rsx", ERREUR),
        ("depots/error.rsx", ERREUR),
        ("depots/page.rsx", PAGE),
    ]);
    assert!(
        generated.contains("failed: Some(error_1)"),
        "le plus proche doit gagner : {generated}"
    );
}

#[test]
fn a_page_with_no_error_file_above_it_has_none() {
    let generated = table(&[("page.rsx", PAGE)]);
    assert!(generated.contains("failed: None"), "{generated}");
}

#[test]
fn an_error_screen_that_does_not_take_the_reason_is_refused() {
    let error = table_for(Path::new(&tree(&[
        ("error.rsx", "fn Error() -> Element {\n    return <p>raté</p>;\n}\n"),
        ("page.rsx", PAGE),
    ])))
    .expect_err("this cannot be generated");
    assert!(
        error.message.contains("pourquoi") || error.message.contains("reason"),
        "le refus doit dire ce qui manque : {}",
        error.message
    );
}
