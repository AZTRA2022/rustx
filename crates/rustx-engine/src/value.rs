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

/// How deep a prop may be nested.
///
/// A cycle is not a shape, and following one would run until the process
/// died. A real prop — rows of fields, a list of lists — is a handful of
/// levels deep; this is far past that and still finite.
const DEEPEST: usize = 32;

/// What a prop that is nested past the limit is refused with.
fn too_deep() -> NativeError {
    NativeError::new(format!(
        "rustx: a prop is nested more than {DEEPEST} deep — a cycle cannot cross the boundary"
    ))
}

/// Reads a JavaScript value, refusing anything `Value` cannot represent.
pub(crate) fn from_v8(value: &V8Value) -> Result<Value, NativeError> {
    read(value, 0)
}

fn read(value: &V8Value, depth: usize) -> Result<Value, NativeError> {
    if depth > DEEPEST {
        return Err(too_deep());
    }
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
    // An array before an object, because every array is also an object.
    if value.is_array() != 0 {
        let length = value.array_length();
        let mut items = Vec::with_capacity(length.max(0) as usize);
        for at in 0..length {
            let Some(item) = value.value_byindex(at) else {
                items.push(Value::Null);
                continue;
            };
            items.push(read(&item, depth + 1)?);
        }
        return Ok(Value::List(items));
    }
    // A function is code, and code does not leave the process it was
    // compiled in. The same for a date, a promise, a DOM node: what
    // crosses is data.
    if value.is_object() != 0 && value.is_function() == 0 {
        let mut fields = Vec::new();
        let mut keys = cef::CefStringList::default();
        if value.keys(Some(&mut keys)) != 0 {
            // Moved into the iterator, not cloned: cloning a
            // `CefStringList` copies the opaque handle by value and the
            // copy lists nothing, so a well-filled object came back as
            // `{}` with nothing to say why.
            for name in keys {
                let Some(field) = value.value_bykey(Some(&CefString::from(name.as_str()))) else {
                    continue;
                };
                fields.push((name, read(&field, depth + 1)?));
            }
        }
        return Ok(Value::Record(fields));
    }
    Err(NativeError::new(
        "unsupported argument: rustx passes null, booleans, numbers, strings, \
         lists of them and objects of them",
    ))
}

/// Builds the JavaScript value a native function returned.
pub(crate) fn into_v8(value: Value) -> Option<V8Value> {
    match value {
        Value::Null => cef::v8_value_create_null(),
        Value::Bool(b) => cef::v8_value_create_bool(i32::from(b)),
        Value::Number(n) => cef::v8_value_create_double(n),
        Value::String(s) => cef::v8_value_create_string(Some(&CefString::from(s.as_str()))),
        Value::List(items) => {
            let array = cef::v8_value_create_array(items.len() as i32)?;
            for (at, item) in items.into_iter().enumerate() {
                let mut item = into_v8(item)?;
                array.set_value_byindex(at as i32, Some(&mut item));
            }
            Some(array)
        }
        Value::Record(fields) => object_of(&fields),
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
