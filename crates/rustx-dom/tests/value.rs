//! What a prop can be.
//!
//! A chart takes its data; a table takes its rows. A boundary whose
//! props are flat is a boundary no real library can be used across, so
//! a value is a list and a record as well as a number.

use rustx_dom::value::{ToValue, record};
use rustx_dom::Value;

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
    assert_eq!(rows[1], Value::List(vec![Value::Number(2.0), Value::Number(3.0)]));
}

#[test]
fn a_record_keeps_a_key_that_is_not_an_identifier() {
    // `dataKey` is one thing; `data-key` and `aria-label` are props a
    // real library takes and a Rust identifier cannot spell.
    let field = record([("data-key", "at".to_value())]);
    assert_eq!(
        field,
        Value::Record(vec![("data-key".to_string(), Value::String("at".to_string()))])
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
    assert_eq!(String::from("hi").to_value(), Value::String("hi".to_string()));
    assert_eq!([1.0, 2.0].as_slice().to_value(), Value::List(vec![Value::Number(1.0), Value::Number(2.0)]));
}
