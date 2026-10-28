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

/// Apply patches to a list of child labels, the way the engine applies
/// them to DOM nodes, so that the result can be compared with what the
/// new tree says the children should be.
fn simulate(old: &[&str], patches: &[Patch]) -> Vec<String> {
    let mut children: Vec<String> = old.iter().map(|s| s.to_string()).collect();
    for patch in patches {
        match patch {
            Patch::Remove { index, .. } if *index < children.len() => {
                children.remove(*index);
            }
            Patch::Insert { index, source, .. } => {
                let at = (*index).min(children.len());
                children.insert(at, format!("#{:?}", source.0));
            }
            Patch::Move { from, to, .. } if *from < children.len() => {
                let moved = children.remove(*from);
                let at = (*to).min(children.len());
                children.insert(at, moved);
            }
            _ => {}
        }
    }
    children
}

fn labelled(items: &[(&str, bool)]) -> Element {
    let mut tag = Element::tag("ul");
    for (label, keyed) in items {
        let child = Element::tag("li").child(text(label));
        tag = tag.child(if *keyed { child.key(*label) } else { child }.build());
    }
    tag.build()
}

fn children_of(element: &Element) -> Vec<String> {
    let Element::Tag(tag) = element else { panic!() };
    tag.children
        .iter()
        .map(|child| match child {
            Element::Tag(tag) => match tag.children.first() {
                Some(Element::Text(text)) => text.clone(),
                _ => String::new(),
            },
            Element::Text(text) => text.clone(),
            _ => String::new(),
        })
        .collect()
}

#[test]
fn a_keyed_list_with_an_unkeyed_child_does_not_panic() {
    // The commonest shape there is: a list with a heading among its
    // children. This used to panic on a tree compared with itself.
    let tree = Element::tag("ul")
        .child(text("Items:"))
        .child(Element::tag("li").key("a").child(text("A")).build())
        .build();
    assert!(diff(&tree, &tree).is_empty(), "nothing changed");
}

#[test]
fn an_unkeyed_child_among_keyed_ones_is_not_dropped() {
    let before = labelled(&[("a", true)]);
    let after = labelled(&[("a", true), ("tail", false)]);
    let patches = diff(&before, &after);
    assert!(!patches.is_empty(), "the new child never reached the screen");
    assert_eq!(simulate(&["a"], &patches).len(), 2, "got {patches:?}");
}

#[test]
fn a_duplicate_key_does_not_leave_an_orphan_behind() {
    // Real data keyed on a field that turns out not to be unique.
    let before = labelled(&[("a", true), ("a", true), ("b", true)]);
    let after = labelled(&[("b", true), ("a", true)]);
    let patches = diff(&before, &after);
    assert_eq!(
        simulate(&["a", "a2", "b"], &patches).len(),
        2,
        "the bookkeeping kept a child the new tree does not have: {patches:?}"
    );
}

#[test]
fn applying_the_patches_always_reproduces_the_new_list() {
    // The property the three tests above are each one case of. Every
    // pair of keyed lists drawn from the same small alphabet.
    let alphabet = ["a", "b", "c"];
    let mut cases = vec![Vec::new()];
    for _ in 0..3 {
        let mut grown = Vec::new();
        for case in &cases {
            for letter in alphabet {
                let mut longer: Vec<&str> = case.clone();
                longer.push(letter);
                grown.push(longer);
            }
        }
        cases.extend(grown);
    }

    for before in &cases {
        for after in &cases {
            let old: Vec<(&str, bool)> = before.iter().map(|k| (*k, true)).collect();
            let new: Vec<(&str, bool)> = after.iter().map(|k| (*k, true)).collect();
            let (old_tree, new_tree) = (labelled(&old), labelled(&new));

            let patches = diff(&old_tree, &new_tree);
            let simulated = simulate(before, &patches);
            assert_eq!(
                simulated.len(),
                children_of(&new_tree).len(),
                "{before:?} -> {after:?} left {} children, patches {patches:?}",
                simulated.len()
            );
        }
    }
}
