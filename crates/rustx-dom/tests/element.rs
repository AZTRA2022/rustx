//! The tree a component returns.

use rustx_dom::{AttributeValue, Element, Event, Handler, Value, into_element};

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
    tag.handlers[0].1.call(Event::of("click"));
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

// ───────── Un évènement qui traverse la frontière React ─────────
//
// Une balise ordinaire écrite dans un sous-arbre React n'est pas montée
// par le moteur : c'est React qui la fait, et son `onInput` arrive par
// le chemin des props. Ce qu'une closure reçoit doit pourtant être le
// même `Event` que sur une balise montée par le moteur — le `.rsx` est
// le même, la closure est la même, et ce qui change est un ancêtre.

#[test]
fn an_event_crosses_as_a_record_and_comes_back_whole() {
    let event = Event {
        kind: "input".to_string(),
        value: "tokio".to_string(),
        id: "recherche".to_string(),
        key: "o".to_string(),
        checked: true,
        shift: false,
        control: true,
        alt: false,
        meta: false,
    };
    assert_eq!(Event::from_value(&event.as_value()), event);
}

#[test]
fn a_value_that_is_not_an_event_reads_as_an_empty_one() {
    // Ce qu'une bibliothèque passe à `onChange` n'est pas toujours un
    // évènement : refuser serait une bibliothèque qui s'arrête, donc
    // c'est un évènement vide, et le texte reçu est quand même là.
    let said = Event::from_value(&Value::String("tokio".to_string()));
    assert_eq!(said.value, "tokio", "le texte passé doit rester lisible");
    assert!(said.kind.is_empty());
    assert_eq!(Event::from_value(&Value::Null), Event::default());
}
