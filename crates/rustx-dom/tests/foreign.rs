//! A node React owns.
//!
//! The boundary is a container: rustx puts it on the page, says which
//! React component goes in it and with what props, and then does not
//! look inside. What React does in there is React's, and a diff that
//! descended into it would be two reconcilers fighting over one subtree.

use rustx_dom::diff::{Patch, Path, diff};
use rustx_dom::{Element, Value};

fn chart(series: &str) -> Element {
    Element::Foreign {
        component: "Chart".to_string(),
        props: vec![("series".to_string(), Value::String(series.to_string()))],
        id: "rustx-js-3-12".to_string(),
    }
}

fn table(series: &str) -> Element {
    Element::Foreign {
        component: "Table".to_string(),
        props: vec![("series".to_string(), Value::String(series.to_string()))],
        id: "rustx-js-3-12".to_string(),
    }
}

#[test]
fn two_foreign_nodes_of_the_same_component_with_the_same_props_are_one_patch_free_node() {
    assert!(diff(&chart("a"), &chart("a")).is_empty());
}

#[test]
fn changing_a_prop_patches_the_props_and_nothing_else() {
    assert_eq!(
        diff(&chart("a"), &chart("b")),
        vec![Patch::ForeignProps {
            path: Path::root(),
            props: vec![("series".to_string(), Value::String("b".to_string()))],
        }]
    );
}

#[test]
fn changing_the_component_replaces_the_container() {
    assert!(matches!(
        diff(&chart("a"), &table("a")).first(),
        Some(Patch::Replace { .. })
    ));
}

#[test]
fn the_diff_never_descends_into_a_foreign_node() {
    // Even when the trees around it differ, no patch may name a path
    // below the container.
    let before = Element::tag("div").child(chart("a")).build();
    let after = Element::tag("div").child(chart("b")).build();
    let patches = diff(&before, &after);
    assert!(
        patches.iter().all(|patch| patch.path().depth() <= 1),
        "got {patches:?}"
    );
}

#[test]
fn a_foreign_node_that_leaves_the_tree_is_a_removal_of_its_container() {
    assert!(matches!(
        diff(&chart("a"), &Element::Empty).first(),
        Some(Patch::Replace { .. })
    ));
}

#[test]
fn a_prop_that_was_dropped_is_a_prop_patch_of_what_is_left() {
    // The props are sent whole, not as a difference: React is given the
    // props of this render, so a prop that went away is simply absent.
    let before = Element::Foreign {
        component: "Chart".to_string(),
        props: vec![
            ("series".to_string(), Value::String("a".to_string())),
            ("live".to_string(), Value::Bool(true)),
        ],
        id: "rustx-js-3-12".to_string(),
    };
    assert_eq!(
        diff(&before, &chart("a")),
        vec![Patch::ForeignProps {
            path: Path::root(),
            props: vec![("series".to_string(), Value::String("a".to_string()))],
        }]
    );
}
