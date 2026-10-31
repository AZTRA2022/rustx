//! What crosses between Rust and JavaScript.
//!
//! Deliberately small: null, a boolean, a number, a string. Nothing
//! borrowed, nothing with a lifetime, nothing holding a `Drop` — a value
//! that crosses the boundary leaves Rust's ownership behind, so the only
//! things allowed across are the ones that can be copied in full.
//!
//! It lives here rather than in the engine because an [`crate::Element`]
//! carries it now: the props of a node React owns are values, and
//! `rustx-dom` cannot depend on the engine. `rustx_engine::Value` is a
//! re-export of this type, so nothing that used it changed.

/// A value a native function can take or return, and a prop a React
/// component can be given.
///
/// A chart takes its data and a table takes its rows, so a prop that
/// can only be a number is a prop no real library can be used with:
/// `List` and `Record` are what a page hands a library, and they are
/// built in the render process as V8 values, never as text.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<Value>),
    /// Fields in the order they were written. A key is whatever the
    /// library asked for — `dataKey`, but also `data-key` or
    /// `aria-label`, which no Rust identifier can spell.
    Record(Vec<(String, Value)>),
}

/// Anything that can be written as a prop.
///
/// The developer writes the value they already have. What they must not
/// have to write is the conversion, which is why this is a trait with
/// the ordinary types done and a `derive` for their own.
pub trait ToValue {
    fn to_value(&self) -> Value;
}

/// A record, written as its fields.
///
/// ```ignore
/// record([("at", 0.to_value()), ("value", 12.5.to_value())])
/// ```
pub fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    )
}

impl ToValue for Value {
    fn to_value(&self) -> Value {
        self.clone()
    }
}

impl ToValue for bool {
    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }
}

impl ToValue for str {
    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

impl ToValue for String {
    fn to_value(&self) -> Value {
        Value::String(self.clone())
    }
}

/// Every number is a JavaScript number, which is an `f64`. A `u64`
/// beyond 2^53 loses precision crossing the boundary, as it does in
/// every other language that talks to JavaScript.
macro_rules! numbers {
    ($($number:ty),*) => {
        $(impl ToValue for $number {
            fn to_value(&self) -> Value {
                Value::Number(*self as f64)
            }
        })*
    };
}

numbers!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64);

impl<T: ToValue> ToValue for Option<T> {
    fn to_value(&self) -> Value {
        match self {
            Some(value) => value.to_value(),
            None => Value::Null,
        }
    }
}

impl<T: ToValue> ToValue for Vec<T> {
    fn to_value(&self) -> Value {
        Value::List(self.iter().map(ToValue::to_value).collect())
    }
}

impl<T: ToValue> ToValue for [T] {
    fn to_value(&self) -> Value {
        Value::List(self.iter().map(ToValue::to_value).collect())
    }
}

impl<T: ToValue + ?Sized> ToValue for &T {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}
