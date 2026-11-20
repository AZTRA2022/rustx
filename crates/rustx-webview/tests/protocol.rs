//! What the page says to the host, and the values it carries.

use rustx_dom::Value;
use rustx_webview::protocol::{
    FromPage, WireError, decode, encode_reply, json_to_value, value_to_json,
};
use serde_json::{Value as Json, json};

fn parsed(text: &str) -> Json {
    serde_json::from_str(text).expect("the host writes JSON")
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

#[test]
fn a_ready_message_carries_the_ids_the_document_had() {
    let ready = decode(r#"{"t":"ready","ids":["app","nav"]}"#).expect("decodes");
    assert_eq!(ready, FromPage::Ready { ids: vec!["app".to_string(), "nav".to_string()] });
    assert!(matches!(decode(r#"{"t":"ready","ids":[1]}"#), Err(WireError::Malformed(_))));
}

#[test]
fn an_event_names_its_node_its_kind_and_the_numbered_input() {
    let event = decode(
        r#"{"t":"event","node":4,"name":"input","seq":7,"event":{"kind":"input","value":"ab"}}"#,
    )
    .expect("decodes");
    let FromPage::Event { node, name, seq, event } = event else {
        panic!("an event was sent");
    };
    assert_eq!((node, name.as_str(), seq), (4, "input", 7));
    assert_eq!(event.get("value"), Some(&Value::String("ab".to_string())));

    // A click has no sequence number.
    let click = decode(r#"{"t":"event","node":1,"name":"click","event":{}}"#).expect("decodes");
    assert!(matches!(click, FromPage::Event { seq: 0, .. }));
}

#[test]
fn a_command_the_page_could_not_run_is_reported() {
    let error = decode(r#"{"t":"error","message":"boom","op":"ap"}"#).expect("decodes");
    assert_eq!(error, FromPage::Error { message: "boom".to_string() });
}