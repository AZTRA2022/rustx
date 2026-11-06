//! What an editor is offered inside the markup.
//!
//! The half of completion the server answers itself: a tag name, a
//! closing tag, a component of the file, a component somebody installed.
//! No compiler runs, no `rust-analyzer` is asked — a keystroke does not
//! wait for either, and a developer with neither installed still gets
//! their tags.

use std::time::Instant;

use rustx_lsp::inside::{self, Place};
use rustx_lsp::suggest::{self, Kind, Page};

/// Where the `|` is, as a one-based line and column, and the text
/// without it. A test that counts columns by hand counts them wrong.
fn cursor(marked: &str) -> (String, usize, usize) {
    let at = marked.find('|').expect("a test marks the cursor with `|`");
    let before = &marked[..at];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rfind('\n')
        .map_or(before.chars().count(), |newline| {
            before[newline + 1..].chars().count()
        })
        + 1;
    (marked.replace('|', ""), line, column)
}

fn place(marked: &str) -> Place {
    let (source, line, column) = cursor(marked);
    inside::at(&source, line, column)
}

fn empty_page() -> Page {
    Page {
        components: Vec::new(),
        libraries: Vec::new(),
    }
}

fn labels(items: &[suggest::Item]) -> Vec<String> {
    items.iter().map(|item| item.label.clone()).collect()
}

#[test]
fn a_half_written_tag_is_a_tag_and_not_an_attribute() {
    // Review Focus 1: a tag being typed in the middle of a page full of
    // valid markup. What is wanted is a tag name; an attribute list
    // would be a hundred wrong answers.
    let marked = "fn Page() -> Element {\n    return <main>\n        <di|\n    </main>;\n}";
    match place(marked) {
        Place::TagName { typed } => assert_eq!(typed, "di"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn every_html_tag_is_offered_and_nothing_else_is() {
    let items = suggest::tags("di", &empty_page());
    let got = labels(&items);
    assert!(got.contains(&"div".to_string()), "got {got:?}");
    assert!(got.contains(&"dialog".to_string()), "got {got:?}");
    assert!(
        !got.iter().any(|label| label.starts_with("on")),
        "an event is not a tag: {got:?}"
    );
    assert!(
        items.iter().all(|item| item.kind == Kind::Tag),
        "something that is not a tag was offered"
    );
}

#[test]
fn a_component_of_the_file_is_offered_by_its_name() {
    let page = Page {
        components: vec![
            ("Header".to_string(), vec!["title: String".to_string()]),
            ("Footer".to_string(), Vec::new()),
        ],
        libraries: Vec::new(),
    };
    let got = labels(&suggest::tags("He", &page));
    assert!(got.contains(&"Header".to_string()), "got {got:?}");
    assert!(
        !got.contains(&"Footer".to_string()),
        "a name that does not match was offered: {got:?}"
    );
    let header = suggest::tags("He", &page)
        .into_iter()
        .find(|item| item.label == "Header")
        .expect("Header");
    assert_eq!(header.kind, Kind::Component);
    assert!(
        header.detail.contains("title"),
        "a component's parameters belong in what the editor shows: {}",
        header.detail
    );
}

#[test]
fn an_installed_component_is_offered_by_its_name() {
    // `npx shadcn add button` leaves `components/ui/button.tsx`, which
    // exports `Button`. A page writes `<Button>`, so the editor offers
    // it.
    let page = Page {
        components: Vec::new(),
        libraries: vec![from("Button", "ui"), from("Card", "ui")],
    };
    let got = labels(&suggest::tags("Bu", &page));
    assert!(got.contains(&"Button".to_string()), "got {got:?}");
}

#[test]
fn a_closing_tag_closes_what_is_open() {
    let marked = "fn Page() -> Element {\n    return <main>\n        <div>text</|\n}";
    match place(marked) {
        Place::ClosingTag { open } => assert_eq!(open, "div"),
        other => panic!("got {other:?}"),
    }
    let items = suggest::closing("div");
    assert_eq!(labels(&items), vec!["div>".to_string()]);
}

#[test]
fn a_cursor_in_plain_rust_is_not_the_markups_business() {
    let marked = "fn Page() -> Element {\n    let x = poi|\n    return <main />;\n}";
    assert!(
        matches!(place(marked), Place::Rust),
        "got {:?}",
        place(marked)
    );
}

#[test]
fn a_thousand_line_page_answers_in_a_blink() {
    // A keystroke does not wait: what the server answers itself is
    // computed on the text the editor just sent, with no compiler in
    // the way.
    let mut source = String::from("fn Page() -> Element {\n    return <main>\n");
    for _ in 0..1_000 {
        source.push_str("        <div className=\"row\">a line of a page</div>\n");
    }
    source.push_str("        <di\n    </main>;\n}\n");
    let line = source.lines().count() - 2;

    let started = Instant::now();
    let found = inside::at(&source, line, 12);
    let items = suggest::tags("di", &empty_page());
    let took = started.elapsed();

    assert!(matches!(found, Place::TagName { .. }), "got {found:?}");
    assert!(!items.is_empty());
    assert!(
        took.as_millis() < 50,
        "a keystroke waited {} ms on a thousand-line page",
        took.as_millis()
    );
}

#[test]
fn a_tag_that_was_never_closed_does_not_take_the_server_down() {
    // Which is the state of a `.rsx` most of the time.
    for marked in [
        "fn Page() -> Element {\n    return <|",
        "fn Page() -> Element {\n    return <div |",
        "fn Page() -> Element {\n    return <div class|",
        "fn Page() -> Element {\n    return <div>{|",
        "|",
    ] {
        let _ = place(marked);
    }
}

#[test]
fn a_components_own_parameters_are_the_attributes_offered() {
    // Review Focus 3. A component takes what its function takes, and
    // nothing else — those are the only names the compiler will accept,
    // so offering `className` on it would be offering an error.
    let page = Page {
        components: vec![(
            "Stat".to_string(),
            vec!["number: String".to_string(), "unit: String".to_string()],
        )],
        libraries: Vec::new(),
    };
    let items = suggest::attributes("Stat", "", &page, &[]);
    let got = labels(&items);
    assert_eq!(
        got,
        vec!["number".to_string(), "unit".to_string()],
        "got {got:?}"
    );
    let number = items.first().expect("number");
    assert_eq!(number.kind, Kind::Property);
    assert_eq!(number.detail, "String");
    assert_eq!(
        number.insert.as_deref(),
        Some("number={}"),
        "a prop is written `name={{…}}`, so that is what is inserted"
    );
}

#[test]
fn an_attribute_already_written_is_not_offered_twice() {
    let page = empty_page();
    let got = labels(&suggest::attributes("div", "cl", &page, &[]));
    assert!(got.contains(&"className".to_string()), "got {got:?}");
    let again = labels(&suggest::attributes(
        "div",
        "cl",
        &page,
        &["className".to_string()],
    ));
    assert!(
        !again.contains(&"className".to_string()),
        "it was offered a second time: {again:?}"
    );
}

#[test]
fn an_event_is_offered_with_the_shape_of_its_handler() {
    let items = suggest::attributes("button", "onCl", &empty_page(), &[]);
    let click = items
        .iter()
        .find(|item| item.label == "onClick")
        .expect("onClick");
    assert_eq!(click.kind, Kind::Event);
    assert_eq!(
        click.insert.as_deref(),
        Some("onClick={move |_| }"),
        "what a developer has to type next is a closure, so it is written for them"
    );
}

#[test]
fn inside_a_style_object_the_keys_are_css_and_not_attributes() {
    // Review Focus 4. `style={{ … }}` is a CSS object; `onClick` in
    // there is a mistake nobody would catch by reading.
    let marked = "fn P() -> Element {\n    return <div style={{ back| }}>a</div>;\n}";
    match place(marked) {
        Place::StyleKey { typed } => assert_eq!(typed, "back"),
        other => panic!("got {other:?}"),
    }
    let got = labels(&suggest::style_keys("back"));
    assert!(got.contains(&"background".to_string()), "got {got:?}");
    assert!(got.contains(&"backgroundColor".to_string()), "got {got:?}");
    assert!(
        !got.iter().any(|label| label.starts_with("on")),
        "an event is not a CSS property: {got:?}"
    );
    let colour = suggest::style_keys("backgroundC")
        .into_iter()
        .next()
        .expect("backgroundColor");
    assert_eq!(colour.insert.as_deref(), Some("backgroundColor: "));
}

#[test]
fn an_installed_component_says_where_it_comes_from() {
    let page = Page {
        components: Vec::new(),
        libraries: vec![from("Button", "ui")],
    };
    let items = suggest::attributes("Button", "class", &page, &[]);
    let class = items
        .iter()
        .find(|item| item.label == "className")
        .expect("className");
    assert!(
        class.detail.contains("library"),
        "a developer has to know the server is guessing: {}",
        class.detail
    );
}

#[test]
fn an_attribute_on_a_plain_tag_is_an_attribute() {
    let marked = "fn P() -> Element {\n    return <div class|\n}";
    match place(marked) {
        Place::AttributeName { tag, typed, .. } => {
            assert_eq!(tag, "div");
            assert_eq!(typed, "class");
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn after_the_last_closing_tag_the_file_is_rust_again() {
    // The bug a real page found: the reader stayed "in the markup" for
    // ever after the first region, so the `{` of the next function's
    // body looked like an expression — and every tag in the rest of the
    // file offered nothing at all.
    let source = "\
fn Page() -> Element {
    return <main>
        <span>text</span>
    </main>;
}

fn Header() -> Element {
    return <header>
        <di
    </header>;
}
";
    match inside::at(source, 9, 12) {
        Place::TagName { typed } => assert_eq!(typed, "di"),
        other => panic!("the second function's markup was not read as markup: {other:?}"),
    }
    // And the plain Rust between the two is plain Rust.
    assert!(matches!(inside::at(source, 7, 10), Place::Rust));
}

#[test]
fn a_tag_that_closes_itself_does_not_end_the_markup_around_it() {
    let source = "\
fn Page() -> Element {
    return <main>
        <br />
        <di
    </main>;
}
";
    match inside::at(source, 4, 12) {
        Place::TagName { typed } => assert_eq!(typed, "di"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn a_nested_region_leaves_the_markup_only_once() {
    // Several regions in one function, and Rust between them.
    let source = "\
fn Page() -> Element {
    let a = <span>one</span>;
    let b = 2;
    return <main>{a}</main>;
}
";
    assert!(
        matches!(inside::at(source, 3, 14), Place::Rust),
        "got {:?}",
        inside::at(source, 3, 14)
    );
}

#[test]
fn a_generic_is_not_a_tag() {
    // `Vec<Point>` in a page's own Rust. Read as a tag, it leaves the
    // reader inside markup for the rest of the file — which is how a
    // real page came to offer nothing from its second function on.
    let source = "\
fn Page() -> Element {
    let data: Vec<Point> = points();
    return <main>
        <di
    </main>;
}
";
    // Right after the `<` of `Vec<`: still Rust.
    assert!(
        matches!(inside::at(source, 2, 19), Place::Rust),
        "got {:?}",
        inside::at(source, 2, 19)
    );
    // And the tag below it is still a tag.
    match inside::at(source, 4, 12) {
        Place::TagName { typed } => assert_eq!(typed, "di"),
        other => panic!("a generic above it broke the markup: {other:?}"),
    }
}

#[test]
fn a_tag_after_a_fat_arrow_or_a_brace_is_a_tag() {
    // The two other places a page writes markup: a closure's body and a
    // branch.
    let source = "\
fn Page() -> Element {
    let rows = names.iter().map(|name| <li>{name}</li>);
    return <main>{rows}</main>;
}
";
    // Just after the `l` of `<li>`: the `<` follows a closure header.
    match inside::at(source, 2, 42) {
        Place::TagName { typed } => assert_eq!(typed, "l"),
        other => panic!("got {other:?}"),
    }
}

/// One name a library exports, as the server reads them off disk.
fn from(name: &str, library: &str) -> rustx_lsp::libraries::Export {
    rustx_lsp::libraries::Export {
        name: name.to_string(),
        from: library.to_string(),
    }
}

#[test]
fn a_component_of_an_installed_library_says_which_library() {
    // `recharts` exports `LineChart`; the editor offers the bare name,
    // with the library on the line beside it.
    let page = Page {
        components: Vec::new(),
        libraries: vec![from("LineChart", "recharts")],
    };
    let items = suggest::tags("Line", &page);
    let chart = items
        .iter()
        .find(|item| item.label == "LineChart")
        .unwrap_or_else(|| panic!("got {:?}", labels(&items)));
    assert_eq!(chart.detail, "recharts");
}

#[test]
fn a_name_two_libraries_export_offers_the_explicit_form_too() {
    // `Dialog` is in `components/ui` and in `@radix-ui/react-dialog`.
    // The bare name resolves to the application's own; the one who
    // wants the primitive has to be told what to write.
    let page = Page {
        components: Vec::new(),
        libraries: vec![from("Dialog", "ui"), from("Dialog", "radixUiReactDialog")],
    };
    let got = labels(&suggest::tags("Dia", &page));
    assert!(got.contains(&"Dialog".to_string()), "got {got:?}");
    assert!(
        got.contains(&"radixUiReactDialog::Dialog".to_string()),
        "the explicit form is not offered: {got:?}"
    );
}

#[test]
fn a_name_one_library_exports_is_not_repeated_in_full() {
    // Nobody needs `recharts::LineChart` offered when nothing else has
    // a `LineChart`: a list twice as long is a list half as useful.
    let page = Page {
        components: Vec::new(),
        libraries: vec![from("LineChart", "recharts")],
    };
    let got = labels(&suggest::tags("", &page));
    assert!(
        !got.contains(&"recharts::LineChart".to_string()),
        "got {got:?}"
    );
}

#[test]
fn the_explicit_form_is_offered_to_whoever_starts_typing_it() {
    let page = Page {
        components: Vec::new(),
        libraries: vec![from("LineChart", "recharts")],
    };
    let got = labels(&suggest::tags("recharts::", &page));
    assert!(
        got.contains(&"recharts::LineChart".to_string()),
        "got {got:?}"
    );
}

#[test]
fn a_page_is_offered_what_the_application_around_it_installed() {
    // The end-to-end shape, from a path on disk: a project with a
    // `package.json`, a `node_modules`, a `components/ui`, and a page
    // in `app/`. Nothing here mocks the lookup, which is the point —
    // the server read `node_modules` from the application's *parent*
    // for a while, so this list came back empty in every real project
    // and the editor offered HTML tags and nothing else.
    let root = std::env::temp_dir().join(format!(
        "rustx-suggest-installed-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    let written = |at: std::path::PathBuf, what: &str| {
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
        std::fs::write(at, what).expect("a file");
    };
    // What makes a directory an application: a manifest and `app/`.
    written(root.join("Cargo.toml"), "[package]\nname = \"mon-app\"\n");
    written(root.join("app/page.rsx"), "fn Page() -> Element { return <main></main>; }\n");
    written(
        root.join("package.json"),
        "{\"dependencies\": {\"recharts\": \"2.12.7\"}}",
    );
    written(
        root.join("node_modules/recharts/package.json"),
        "{\"name\": \"recharts\", \"types\": \"./index.d.ts\"}",
    );
    written(
        root.join("node_modules/recharts/index.d.ts"),
        "export { LineChart } from './chart/LineChart';\n",
    );
    written(
        root.join("components/ui/card.tsx"),
        "export function Card() { return null }\n",
    );
    written(
        root.join("components/Editor.tsx"),
        "export default function Editor() { return null }\n",
    );

    let source = std::fs::read_to_string(root.join("app/page.rsx")).expect("the page");
    let page = suggest::page_of(&source, &root.join("app/page.rsx"));
    let names: Vec<&str> = page
        .libraries
        .iter()
        .map(|export| export.name.as_str())
        .collect();

    assert!(names.contains(&"LineChart"), "a library off npm: {names:?}");
    assert!(names.contains(&"Card"), "what shadcn add wrote: {names:?}");
    assert!(names.contains(&"Editor"), "a component of its own: {names:?}");

    let _ = std::fs::remove_dir_all(&root);
}
