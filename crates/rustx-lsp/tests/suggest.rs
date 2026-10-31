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
        libraries: vec!["Button".to_string(), "Card".to_string()],
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
