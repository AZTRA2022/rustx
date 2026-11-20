//! The wire between the Rust tree and the page.

use std::cell::RefCell;
use std::rc::Rc;

use rustx_dom::diff::{Patch, Path, diff};
use rustx_dom::{AttributeValue, Callback, Element, ForeignChild, ForeignNode, Handler, Value};
use rustx_webview::protocol::{
    FromPage, Registry, WireError, decode, encode_mount, encode_patches, encode_reply,
    json_to_value, value_to_json,
};
use serde_json::{Value as Json, json};

fn parsed(text: &str) -> Json {
    serde_json::from_str(text).expect("the host writes JSON")
}

fn page() -> Element {
    Element::tag("div")
        .attribute("class", AttributeValue::Text("a".to_string()))
        .child(Element::tag("p").child(Element::Text("one".to_string())).build())
        .build()
}

#[test]
fn every_patch_variant_lowers_to_an_operation() {
    let tree = page();
    let foreign = ForeignNode {
        component: "Chart".to_string(),
        props: vec![("n".to_string(), Value::Number(2.0))],
        callbacks: vec![("onPick".to_string(), Callback::new(|_| {}))],
        children: vec![
            ForeignChild::Text("t".to_string()),
            ForeignChild::Value(Value::Bool(true)),
        ],
    };
    let cases = [
        (
            Patch::SetText { path: Path(vec![0, 0]), text: "x".to_string() },
            "text",
        ),
        (
            Patch::SetAttribute {
                path: Path(vec![]),
                name: "id".to_string(),
                value: AttributeValue::Flag(true),
            },
            "attr",
        ),
        (
            Patch::RemoveAttribute { path: Path(vec![]), name: "id".to_string() },
            "unattr",
        ),
        (Patch::Replace { path: Path(vec![0]), source: Path(vec![0]) }, "replace"),
        (
            Patch::Insert { parent: Path(vec![]), index: 1, source: Path(vec![0]) },
            "insert",
        ),
        (Patch::Remove { parent: Path(vec![]), index: 0 }, "remove"),
        (Patch::Move { parent: Path(vec![]), from: 0, to: 1 }, "move"),
        (Patch::ReplaceHandlers { path: Path(vec![]), source: Path(vec![]) }, "handlers"),
        (Patch::ForeignProps { path: Path(vec![0]), node: foreign }, "foreign"),
    ];
    for (patch, expected) in cases {
        let mut registry = Registry::new();
        let text = encode_patches(&[patch], &tree, &mut registry).expect("lowers");
        let message = parsed(&text);
        assert_eq!(message["t"], "patches");
        assert_eq!(message["batch"][0]["op"], expected, "{text}");
    }
}

#[test]
fn a_real_diff_is_carried_whole() {
    let old = page();
    let new = Element::tag("div")
        .attribute("class", AttributeValue::Text("b".to_string()))
        .child(Element::tag("p").child(Element::Text("two".to_string())).build())
        .child(Element::tag("p").child(Element::Text("three".to_string())).build())
        .build();
    let batch = diff(&old, &new);
    assert!(batch.len() >= 3, "{batch:?}");
    let text = encode_patches(&batch, &new, &mut Registry::new()).expect("lowers");
    assert_eq!(parsed(&text)["batch"].as_array().map(Vec::len), Some(batch.len()));
}

#[test]
fn an_empty_batch_is_an_empty_list() {
    let text = encode_patches(&[], &Element::Empty, &mut Registry::new()).expect("lowers");
    assert_eq!(parsed(&text), json!({ "t": "patches", "batch": [] }));
}

#[test]
fn a_patch_naming_a_place_the_tree_lacks_is_refused() {
    let batch = [Patch::Replace { path: Path(vec![]), source: Path(vec![5, 5]) }];
    let error = encode_patches(&batch, &page(), &mut Registry::new()).expect_err("no such place");
    assert_eq!(error, WireError::BadPath(vec![5, 5]));
}

#[test]
fn a_handler_runs_by_the_id_the_page_was_given() {
    let seen = Rc::new(RefCell::new(String::new()));
    let kept = Rc::clone(&seen);
    let tree = Element::tag("button")
        .handler("onClick", Handler::new(move |event| *kept.borrow_mut() = event.value))
        .build();
    let mut registry = Registry::new();
    let message = parsed(&encode_mount(&tree, &mut registry).expect("mounts"));
    let id = message["node"]["id"].as_u64().expect("a tag has an id");
    assert_eq!(message["node"]["on"], json!(["onClick"]));

    let click = decode(&format!(
        r#"{{"t":"event","node":{id},"name":"onClick","event":{{"kind":"click","value":"hi"}}}}"#
    ))
    .expect("decodes");
    let FromPage::Event { node, name, event } = click else {
        panic!("an event was sent");
    };
    assert!(registry.dispatch(node, &name, &event));
    assert_eq!(*seen.borrow(), "hi");
}

#[test]
fn an_event_for_a_node_that_is_gone_is_ignored() {
    let tree = Element::tag("button").handler("onClick", Handler::new(|_| {})).build();
    let mut registry = Registry::new();
    let id = parsed(&encode_mount(&tree, &mut registry).expect("mounts"))["node"]["id"]
        .as_u64()
        .expect("an id");
    registry.forget(id);
    assert!(!registry.dispatch(id, "onClick", &Value::Null));
    assert!(!registry.dispatch(9999, "onClick", &Value::Null));
    assert!(!registry.dispatch(id, "onNothing", &Value::Null));
}

#[test]
fn a_callback_given_to_a_library_runs_by_id() {
    let got = Rc::new(RefCell::new(Vec::new()));
    let kept = Rc::clone(&got);
    let node = ForeignNode {
        component: "Tabs".to_string(),
        props: vec![],
        callbacks: vec![(
            "onValueChange".to_string(),
            Callback::new(move |arguments| kept.borrow_mut().extend_from_slice(arguments)),
        )],
        children: vec![],
    };
    let tree = Element::Foreign { node, id: "rustx-js-1-1".to_string() };
    let mut registry = Registry::new();
    let message = parsed(&encode_mount(&tree, &mut registry).expect("mounts"));
    let id = message["node"]["node"]["callbacks"][0][1].as_u64().expect("an id");
    assert!(registry.call_back(id, &[Value::String("two".to_string())]));
    assert_eq!(*got.borrow(), vec![Value::String("two".to_string())]);
    assert!(!registry.call_back(id + 100, &[]));
}

#[test]
fn awkward_text_survives_the_trip() {
    let text = "say \"hi\"\n\ttab \\ slash \u{2028}\u{2029} \u{1F980} \0 end";
    let value = Value::Record(vec![("t".to_string(), Value::String(text.to_string()))]);
    let json = value_to_json(&value, 0).expect("writes");
    let written = serde_json::to_string(&json).expect("serializes");
    let back = json_to_value(&parsed(&written), 0).expect("reads");
    assert_eq!(back, value);
}

#[test]
fn nan_and_infinity_are_refused_by_name() {
    for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            value_to_json(&Value::Number(number), 0).expect_err("not JSON"),
            WireError::NotANumber
        );
    }
}

#[test]
fn a_value_nested_too_deep_is_refused() {
    let mut value = Value::Null;
    for _ in 0..200 {
        value = Value::List(vec![value]);
    }
    assert_eq!(value_to_json(&value, 0).expect_err("too deep"), WireError::TooDeep);
}

#[test]
fn a_message_nested_too_deep_is_refused_not_a_crash() {
    let deep = format!(
        r#"{{"t":"callback","id":1,"args":[{}0{}]}}"#,
        "[".repeat(300),
        "]".repeat(300)
    );
    assert_eq!(decode(&deep).expect_err("too deep"), WireError::TooDeep);
}

#[test]
fn what_cannot_be_read_is_a_typed_error() {
    assert_eq!(
        decode(r#"{"t":"mystery"}"#).expect_err("unknown"),
        WireError::UnknownTag("mystery".to_string())
    );
    for broken in [
        r#"{"t":"event","node":1,"name":"onClick","ev"#,
        "",
        "[]",
        r#"{"t":"event","node":"one","name":"x","event":{}}"#,
        r#"{"t":"call","id":1,"name":"f"}"#,
        r#"{"t":7}"#,
    ] {
        assert!(
            matches!(decode(broken), Err(WireError::Malformed(_))),
            "{broken:?} should be malformed"
        );
    }
}

#[test]
fn a_call_decodes_and_its_reply_names_the_same_id() {
    let call = decode(r#"{"t":"call","id":7,"name":"add","args":[1,2]}"#).expect("decodes");
    assert_eq!(
        call,
        FromPage::Call {
            id: 7,
            name: "add".to_string(),
            args: vec![Value::Number(1.0), Value::Number(2.0)]
        }
    );
    let ok = parsed(&encode_reply(7, Ok(Value::Number(3.0))).expect("writes"));
    assert_eq!(ok, json!({ "t": "reply", "id": 7, "ok": 3.0 }));
    let failed = parsed(&encode_reply(7, Err("no".to_string())).expect("writes"));
    assert_eq!(failed, json!({ "t": "reply", "id": 7, "err": "no" }));
}
