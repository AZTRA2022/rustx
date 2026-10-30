//! The values that cross between Rust and JavaScript, and how they are
//! read from V8 and built back into it.
//!
//! The type itself lives in `rustx-dom`, because an `Element` carries it
//! now: the props of a node React owns are values. What stays here is
//! the V8 half, which is the engine's business — `rustx_engine::Value`
//! is a re-export, so nothing that used it changed.

use cef::{CefString, ImplV8Value, V8Value};

pub use rustx_dom::Value;

/// What a native function says when it cannot do what was asked.
///
/// The message reaches JavaScript as the text of a thrown exception, so it
/// is written for whoever is reading the console.
#[derive(Debug, Clone)]
pub struct NativeError {
    pub message: String,
}

impl NativeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for NativeError {}

/// Reads a JavaScript value, refusing anything `Value` cannot represent.
pub(crate) fn from_v8(value: &V8Value) -> Result<Value, NativeError> {
    if value.is_null() != 0 || value.is_undefined() != 0 {
        return Ok(Value::Null);
    }
    if value.is_bool() != 0 {
        return Ok(Value::Bool(value.bool_value() != 0));
    }
    if value.is_int() != 0 {
        return Ok(Value::Number(f64::from(value.int_value())));
    }
    if value.is_double() != 0 {
        return Ok(Value::Number(value.double_value()));
    }
    if value.is_string() != 0 {
        return Ok(Value::String(
            CefString::from(&value.string_value()).to_string(),
        ));
    }
    Err(NativeError::new(
        "unsupported argument: rustx passes null, booleans, numbers and strings",
    ))
}

/// Builds the JavaScript value a native function returned.
pub(crate) fn into_v8(value: Value) -> Option<V8Value> {
    match value {
        Value::Null => cef::v8_value_create_null(),
        Value::Bool(b) => cef::v8_value_create_bool(i32::from(b)),
        Value::Number(n) => cef::v8_value_create_double(n),
        Value::String(s) => cef::v8_value_create_string(Some(&CefString::from(s.as_str()))),
    }
}

/// The JavaScript object a node React owns is given as its props.
pub(crate) fn object_of(props: &[(String, Value)]) -> Option<V8Value> {
    let object = cef::v8_value_create_object(None, None)?;
    for (name, value) in props {
        let mut value = into_v8(value.clone())?;
        object.set_value_bykey(
            Some(&CefString::from(name.as_str())),
            Some(&mut value),
            cef::V8Propertyattribute::default(),
        );
    }
    Some(object)
}
