//! The tree a component returns.

use rustx_dom::{AttributeValue, Element, Event, Handler, into_element};

#[test]
fn a_tag_carries_its_attributes_handlers_and_children_in_order() {
    let element = Element::tag("div")
        .attribute("className", AttributeValue::Text("row".into()))
        .child(Element::Text("one".into()))
        .child(Element::Text("two".into()))
        .build();

    let Element::Tag(tag) = element else {
        panic!("expected a tag")
    };
    assert_eq!(tag.tag, "div");
    assert_eq!(tag.attributes.len(), 1);
    assert_eq!(tag.children.len(), 2);
    assert!(matches!(&tag.children[0], Element::Text(t) if t == "one"));
}

#[test]
fn a_handler_is_callable_and_needs_no_thread_bound() {
    // A handler captures an `Rc` freely: it never leaves the UI thread.
    let seen = std::rc::Rc::new(std::cell::Cell::new(0));
    let counter = seen.clone();
    let element = Element::tag("button")
        .handler(
            "onClick",
            Handler::new(move |_event| counter.set(counter.get() + 1)),
        )
        .build();

    let Element::Tag(tag) = element else {
        panic!("expected a tag")
    };
    tag.handlers[0].1.call(Event {
        kind: "click".into(),
    });
    assert_eq!(seen.get(), 1);
}

#[test]
fn anything_displayable_can_be_written_between_braces() {
    // `{clicks}` — no `.to_string()`, no `.get()`, no conversion at all.
    assert!(matches!(into_element(7_u32), Element::Text(t) if t == "7"));
    assert!(matches!(into_element("hi"), Element::Text(t) if t == "hi"));
    assert!(matches!(into_element(Element::Empty), Element::Empty));
    assert!(matches!(into_element(Vec::<Element>::new()), Element::Fragment(c) if c.is_empty()));
}
