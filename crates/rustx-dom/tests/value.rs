//! What a prop can be.
//!
//! A chart takes its data; a table takes its rows. A boundary whose
//! props are flat is a boundary no real library can be used across, so
//! a value is a list and a record as well as a number.

use rustx_dom::Value;
use rustx_dom::value::{ToValue, record};

#[test]
fn a_list_of_records_is_a_value() {
    let points = vec![
        record([("at", 0.0.to_value()), ("value", 0.0.to_value())]),
        record([("at", 1.0.to_value()), ("value", 1.0.to_value())]),
    ];
    let Value::List(rows) = points.to_value() else {
        panic!("a list of records should be a list");
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[1],
        Value::Record(vec![
            ("at".to_string(), Value::Number(1.0)),
            ("value".to_string(), Value::Number(1.0)),
        ])
    );
}

#[test]
fn an_empty_list_is_a_list_not_null() {
    // `data={vec![]}` is a chart with no points yet, which a library
    // draws as an empty chart. Handed over as `null` it is an argument
    // about the library's own defaults instead.
    let empty: Vec<f64> = Vec::new();
    assert_eq!(empty.to_value(), Value::List(Vec::new()));
}

#[test]
fn a_list_of_lists_does_not_flatten() {
    let nested = vec![vec![1.0], vec![2.0, 3.0]];
    let Value::List(rows) = nested.to_value() else {
        panic!("a list of lists should be a list");
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[1],
        Value::List(vec![Value::Number(2.0), Value::Number(3.0)])
    );
}

#[test]
fn a_record_keeps_a_key_that_is_not_an_identifier() {
    // `dataKey` is one thing; `data-key` and `aria-label` are props a
    // real library takes and a Rust identifier cannot spell.
    let field = record([("data-key", "at".to_value())]);
    assert_eq!(
        field,
        Value::Record(vec![(
            "data-key".to_string(),
            Value::String("at".to_string())
        )])
    );
}

#[test]
fn an_option_is_null_or_the_value() {
    assert_eq!(None::<f64>.to_value(), Value::Null);
    assert_eq!(Some(2u8).to_value(), Value::Number(2.0));
}

#[test]
fn the_ordinary_types_cross_as_themselves() {
    assert_eq!(true.to_value(), Value::Bool(true));
    assert_eq!(3i64.to_value(), Value::Number(3.0));
    assert_eq!("hi".to_value(), Value::String("hi".to_string()));
    assert_eq!(
        String::from("hi").to_value(),
        Value::String("hi".to_string())
    );
    assert_eq!(
        [1.0, 2.0].as_slice().to_value(),
        Value::List(vec![Value::Number(1.0), Value::Number(2.0)])
    );
}

// ───────────────────── Lire ce qui a traversé ─────────────────────

#[test]
fn a_string_is_read_as_a_string_and_nothing_else_is() {
    // Pourquoi ceci existe : une bibliothèque passe une valeur à un
    // callback, et la lire demandait un `match` sur cinq variantes à
    // chaque `onValueChange` -- ou, en pratique, d'y renoncer.
    assert_eq!(Value::String("rust".into()).as_str(), Some("rust"));
    assert_eq!(Value::Number(7.0).as_str(), None);
    assert_eq!(Value::Null.as_str(), None);
}

#[test]
fn a_number_written_as_text_is_still_a_number_when_asked_for_one() {
    // Une bibliothèque qui passe `"42"` là où sa documentation dit un
    // nombre est une bibliothèque avec laquelle il faut travailler
    // quand même.
    assert_eq!(Value::String("42".into()).number(), Some(42.0));
    assert_eq!(Value::Number(42.0).number(), Some(42.0));
    assert_eq!(Value::String("".into()).number(), None);
    assert_eq!(Value::String("abc".into()).number(), None);
}

#[test]
fn a_whole_number_is_written_without_a_decimal_point() {
    // Un curseur qui passe 7 ne doit pas afficher « 7.0 ».
    assert_eq!(Value::Number(7.0).text(), "7");
    assert_eq!(Value::Number(7.5).text(), "7.5");
    assert_eq!(Value::Null.text(), "");
    assert_eq!(Value::Bool(true).text(), "true");
}

#[test]
fn a_field_of_a_record_is_read_by_name_and_an_item_by_its_rank() {
    let value = Value::Record(vec![
        ("nom".into(), Value::String("rust".into())),
        (
            "etoiles".into(),
            Value::List(vec![Value::Number(1.0), Value::Number(2.0)]),
        ),
    ]);
    assert_eq!(value.get("nom").and_then(Value::as_str), Some("rust"));
    assert_eq!(
        value.at(&["etoiles", "1"]).and_then(Value::as_f64),
        Some(2.0)
    );
    assert!(value.get("rien").is_none());
    assert!(value.at(&["nom", "rien"]).is_none());
}

#[test]
fn what_means_yes_is_what_javascript_means_by_it() {
    assert!(!Value::Null.truth());
    assert!(!Value::Number(0.0).truth());
    assert!(!Value::String(String::new()).truth());
    assert!(Value::String("non".into()).truth());
    assert!(Value::Number(1.0).truth());
    assert!(Value::List(Vec::new()).truth(), "une liste vide est un objet");
}

// ---------------------------------------------------------------------
// `style`, on the two sides of the boundary.
//
// The DOM takes a string — `style="width: 50%"` — and React takes an
// object. A tag written inside a library's component is rendered by
// React, so the string has to become one: React throws on a string
// (error #62), the exception took the whole boundary with it, and a
// card with three progress bars in it never appeared. Nothing in the
// page was wrong.
// ---------------------------------------------------------------------

use rustx_dom::value::style_object;

#[test]
fn one_declaration_becomes_one_field() {
    let style = style_object("width: 50%");
    assert_eq!(
        style.get("width").and_then(Value::as_str),
        Some("50%"),
        "got {style:?}"
    );
}

#[test]
fn a_dashed_property_gets_the_name_react_knows() {
    let style = style_object("background-color: red; margin-top: 2px");
    assert_eq!(
        style.get("backgroundColor").and_then(Value::as_str),
        Some("red"),
        "got {style:?}"
    );
    assert_eq!(
        style.get("marginTop").and_then(Value::as_str),
        Some("2px"),
        "got {style:?}"
    );
}

#[test]
fn a_custom_property_keeps_its_two_dashes() {
    // React sets a custom property as it is written, and camel-casing it
    // would set a property nobody declared.
    let style = style_object("--brand: oklch(0.7 0.1 250)");
    assert_eq!(
        style.get("--brand").and_then(Value::as_str),
        Some("oklch(0.7 0.1 250)"),
        "got {style:?}"
    );
}

#[test]
fn a_value_that_holds_a_colon_keeps_it() {
    let style = style_object("background: url(https://example.com/a.png)");
    assert_eq!(
        style.get("background").and_then(Value::as_str),
        Some("url(https://example.com/a.png)"),
        "got {style:?}"
    );
}

#[test]
fn what_is_not_a_declaration_is_left_out() {
    let style = style_object("; width: 2px ;; nonsense ;");
    assert_eq!(
        style.as_record().map(|fields| fields.len()),
        Some(1),
        "got {style:?}"
    );
}

#[test]
fn an_empty_style_is_an_empty_object() {
    // Not null: React takes an object, and an empty one is "no style".
    assert_eq!(style_object("   "), rustx_dom::value::record([]));
}
