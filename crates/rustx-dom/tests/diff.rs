//! Comparing two trees, with no DOM in sight.

use rustx_dom::diff::{Patch, diff};
use rustx_dom::{AttributeValue, Element};

fn text(value: &str) -> Element {
    Element::Text(value.into())
}

fn tree() -> Element {
    Element::tag("div")
        .attribute("class", AttributeValue::Text("row".into()))
        .child(text("one"))
        .child(Element::tag("b").child(text("two")).build())
        .build()
}

fn counter(n: u32) -> Element {
    Element::tag("p").child(text(&n.to_string())).build()
}

fn list(items: &[&str]) -> Element {
    let mut tag = Element::tag("ul");
    for item in items {
        tag = tag.child(Element::tag("li").key(*item).child(text(item)).build());
    }
    tag.build()
}

fn unkeyed(items: &[&str]) -> Element {
    let mut tag = Element::tag("ul");
    for item in items {
        tag = tag.child(Element::tag("li").child(text(item)).build());
    }
    tag.build()
}

#[test]
fn an_unchanged_tree_produces_no_patch() {
    assert!(diff(&tree(), &tree()).is_empty());
}

#[test]
fn only_the_text_that_changed_is_patched() {
    let patches = diff(&counter(0), &counter(1));
    assert_eq!(patches.len(), 1, "got {patches:?}");
    assert!(matches!(&patches[0], Patch::SetText { text, .. } if text == "1"));
}

#[test]
fn a_different_tag_is_replaced_whole() {
    let patches = diff(&Element::tag("p").build(), &Element::tag("div").build());
    assert!(matches!(patches.as_slice(), [Patch::Replace { .. }]), "got {patches:?}");
}

#[test]
fn an_attribute_that_changed_is_set_and_one_that_went_is_removed() {
    let before = Element::tag("div")
        .attribute("class", AttributeValue::Text("a".into()))
        .attribute("id", AttributeValue::Text("x".into()))
        .build();
    let after = Element::tag("div")
        .attribute("class", AttributeValue::Text("b".into()))
        .build();
    let patches = diff(&before, &after);
    assert!(
        patches.iter().any(|p| matches!(p, Patch::SetAttribute { name, .. } if name == "class")),
        "got {patches:?}"
    );
    assert!(
        patches.iter().any(|p| matches!(p, Patch::RemoveAttribute { name, .. } if name == "id")),
        "got {patches:?}"
    );
}

#[test]
fn a_keyed_list_reordered_moves_nodes_instead_of_rewriting_them() {
    // Review Focus 3: state follows the key, not the position.
    let patches = diff(&list(&["a", "b", "c"]), &list(&["c", "a", "b"]));
    assert!(!patches.is_empty(), "nothing happened");
    assert!(
        patches.iter().all(|p| matches!(p, Patch::Move { .. })),
        "got {patches:?}"
    );
}

#[test]
fn a_keyed_item_removed_takes_only_itself() {
    let patches = diff(&list(&["a", "b", "c"]), &list(&["a", "c"]));
    assert!(
        matches!(patches.as_slice(), [Patch::Remove { index: 1, .. }]),
        "got {patches:?}"
    );
}

#[test]
fn a_keyed_item_added_is_inserted_where_it_belongs() {
    let patches = diff(&list(&["a", "c"]), &list(&["a", "b", "c"]));
    assert!(
        patches.iter().any(|p| matches!(p, Patch::Insert { index: 1, .. })),
        "got {patches:?}"
    );
}

#[test]
fn an_unkeyed_list_falls_back_to_position() {
    let patches = diff(&unkeyed(&["a", "b"]), &unkeyed(&["a", "b", "c"]));
    assert!(
        matches!(patches.as_slice(), [Patch::Insert { index: 2, .. }]),
        "got {patches:?}"
    );
}

#[test]
fn a_handler_is_always_replaced_because_a_closure_has_no_equality() {
    let before = Element::tag("button")
        .handler("onClick", rustx_dom::Handler::new(|_| ()))
        .child(text("a"))
        .build();
    let after = Element::tag("button")
        .handler("onClick", rustx_dom::Handler::new(|_| ()))
        .child(text("b"))
        .build();
    let patches = diff(&before, &after);
    assert!(
        patches.iter().any(|p| matches!(p, Patch::ReplaceHandlers { .. })),
        "got {patches:?}"
    );
}
