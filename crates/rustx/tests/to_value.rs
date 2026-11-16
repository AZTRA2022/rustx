//! A structure of the application's own, written as a prop.
//!
//! A chart's data is rows, and rows are a type the developer already
//! has. What they must not have to write is the conversion.

use rustx::{ToValue, Value};

#[derive(ToValue)]
struct Point {
    at: f64,
    value: f64,
}

#[derive(ToValue)]
struct Axis {
    #[rustx(rename = "data-key")]
    data_key: String,
    shown: bool,
}

#[test]
fn a_struct_becomes_a_record_of_its_fields() {
    let point = Point {
        at: 1.0,
        value: 4.0,
    };
    assert_eq!(
        point.to_value(),
        Value::Record(vec![
            ("at".to_string(), Value::Number(1.0)),
            ("value".to_string(), Value::Number(4.0)),
        ])
    );
}

#[test]
fn a_renamed_field_crosses_under_its_new_name() {
    // `data-key` is what the library asks for and no Rust identifier can
    // spell.
    let axis = Axis {
        data_key: "at".to_string(),
        shown: true,
    };
    assert_eq!(
        axis.to_value(),
        Value::Record(vec![
            ("data-key".to_string(), Value::String("at".to_string())),
            ("shown".to_string(), Value::Bool(true)),
        ])
    );
}

#[test]
fn a_list_of_them_is_what_a_chart_is_given() {
    let points = vec![
        Point {
            at: 0.0,
            value: 0.0,
        },
        Point {
            at: 1.0,
            value: 1.0,
        },
    ];
    let Value::List(rows) = points.to_value() else {
        panic!("a list of rows should be a list");
    };
    assert_eq!(rows.len(), 2);
}
