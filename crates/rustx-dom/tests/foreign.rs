//! A node React owns.
//!
//! The boundary is a container: rustx puts it on the page, says which
//! React component goes in it, with what props and with what inside it,
//! and then does not look in. What React does in there is React's, and
//! a diff that descended into it would be two reconcilers fighting over
//! one subtree.

use rustx_dom::diff::{Patch, Path, diff};
use rustx_dom::{Element, ForeignChild, ForeignNode, Value, into_foreign_child};

fn node(component: &str, series: &str) -> ForeignNode {
    ForeignNode {
        component: component.to_string(),
        props: vec![("series".to_string(), Value::String(series.to_string()))],
        callbacks: Vec::new(),
        children: Vec::new(),
    }
}

fn chart(series: &str) -> Element {
    Element::Foreign {
        node: node("recharts::LineChart", series),
        id: "rustx-js-3-12".to_string(),
    }
}

fn table(series: &str) -> Element {
    Element::Foreign {
        node: node("recharts::Table", series),
        id: "rustx-js-3-12".to_string(),
    }
}

/// The same chart, with an axis written inside it.
fn chart_with_axis(key: &str) -> Element {
    let mut chart = node("recharts::LineChart", "a");
    chart.children.push(ForeignChild::Node(ForeignNode {
        component: "recharts::XAxis".to_string(),
        props: vec![("dataKey".to_string(), Value::String(key.to_string()))],
        callbacks: Vec::new(),
        children: Vec::new(),
    }));
    Element::Foreign {
        node: chart,
        id: "rustx-js-3-12".to_string(),
    }
}

#[test]
fn two_foreign_nodes_of_the_same_component_with_the_same_props_are_one_patch_free_node() {
    assert!(diff(&chart("a"), &chart("a")).is_empty());
}

#[test]
fn changing_a_prop_patches_the_node_and_nothing_else() {
    assert_eq!(
        diff(&chart("a"), &chart("b")),
        vec![Patch::ForeignProps {
            path: Path::root(),
            node: node("recharts::LineChart", "b"),
        }]
    );
}

#[test]
fn a_child_of_a_foreign_node_whose_props_changed_is_a_patch() {
    // The boundary is the node and everything under it. An axis whose
    // `dataKey` changed is a change React has to be told about, even
    // though the chart's own props are the same — handed the first
    // render's tree, it would draw the wrong axis for good.
    let patches = diff(&chart_with_axis("at"), &chart_with_axis("value"));
    assert_eq!(patches.len(), 1, "got {patches:?}");
    let Some(Patch::ForeignProps { node, .. }) = patches.first() else {
        panic!("a changed child should patch the boundary: {patches:?}");
    };
    assert_eq!(
        node.children,
        vec![ForeignChild::Node(ForeignNode {
            component: "recharts::XAxis".to_string(),
            props: vec![("dataKey".to_string(), Value::String("value".to_string()))],
            callbacks: Vec::new(),
            children: Vec::new(),
        })]
    );
}

#[test]
fn a_child_that_did_not_change_is_no_patch_at_all() {
    assert!(diff(&chart_with_axis("at"), &chart_with_axis("at")).is_empty());
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

/// A list of rows, written inside a library's component.
///
/// `{items.iter().map(Row).collect::<Vec<_>>()}` is the only way Rust
/// has of rendering the same component several times — `for` is a
/// statement and cannot sit in `{…}` — and it is what the README tells
/// a developer to write. Between a native tag's angle brackets it has
/// always worked, through `IntoElement<IsList>`. Between a library
/// component's it did not: `IntoForeignChild` knew an `Element` and
/// knew a value, and a `Vec<Element>` is neither, so rustc answered
/// "the trait bound `Element: ToValue` is not satisfied" about a
/// generated file — for the one line the documentation recommends.
#[test]
fn a_list_of_elements_may_be_written_inside_a_library_component() {
    let rows = vec![
        Element::tag("li")
            .child(Element::Text("one".to_string()))
            .build(),
        Element::tag("li")
            .child(Element::Text("two".to_string()))
            .build(),
    ];
    assert_eq!(
        into_foreign_child(rows.clone()),
        ForeignChild::Element(Box::new(Element::Fragment(rows))),
    );
}

/// The same for `Option<Element>`, which is what a `.find(…).map(…)`
/// leaves behind, and what `IntoElement` has always accepted beside a
/// native tag.
#[test]
fn an_optional_element_may_be_written_inside_a_library_component() {
    let row = Element::tag("li").build();
    assert_eq!(
        into_foreign_child(Some(row.clone())),
        ForeignChild::Element(Box::new(row)),
    );
    assert_eq!(
        into_foreign_child(None::<Element>),
        ForeignChild::Element(Box::new(Element::Empty)),
    );
}

#[test]
fn a_prop_that_was_dropped_is_a_prop_patch_of_what_is_left() {
    // The node is sent whole, not as a difference: React is given the
    // tree of this render, so a prop that went away is simply absent.
    let before = Element::Foreign {
        node: ForeignNode {
            component: "recharts::LineChart".to_string(),
            props: vec![
                ("series".to_string(), Value::String("a".to_string())),
                ("live".to_string(), Value::Bool(true)),
            ],
            callbacks: Vec::new(),
            children: Vec::new(),
        },
        id: "rustx-js-3-12".to_string(),
    };
    assert_eq!(
        diff(&before, &chart("a")),
        vec![Patch::ForeignProps {
            path: Path::root(),
            node: node("recharts::LineChart", "a"),
        }]
    );
}
