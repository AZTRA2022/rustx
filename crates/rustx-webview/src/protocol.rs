//! The messages the page sends to the host, and the values they carry.
//!
//! Host to page there is one message, the commands of a frame, written by
//! [`crate::dom`]. Page to host, JSON objects with a `"t"`:
//!
//! - `{"t":"ready","ids":[…]}`: the document is there, with these ids.
//! - `{"t":"event","node":id,"name":"click","seq":n,"event":{…}}`: an event
//!   on a node the host built. `seq` numbers the input of a field.
//! - `{"t":"call","id":n,"name":"…","args":[…]}`: a native call.
//! - `{"t":"callback","id":n,"args":[…]}`: a library calling a prop.
//! - `{"t":"error","message":"…"}`: a command the page could not run.
//!
//! Anything that is not one of these is the application's own message.

use rustx_dom::Value;
use serde_json::{Map, Value as Json, json};

/// How deep a value may nest on the wire. serde_json refuses to parse past
/// 128; staying below it means what we write we can read.
const MAX_DEPTH: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum WireError {
    /// Not JSON, truncated, or a field missing or of the wrong kind.
    Malformed(String),
    /// A `"t"` this side does not know.
    UnknownTag(String),
    /// NaN and infinity: JSON has no way to write them.
    NotANumber,
    /// Nested deeper than `MAX_DEPTH`.
    TooDeep,
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "malformed message: {why}"),
            Self::UnknownTag(tag) => write!(f, "unknown message tag `{tag}`"),
            Self::NotANumber => f.write_str("NaN and infinity cannot cross to the page"),
            Self::TooDeep => write!(f, "nested deeper than {MAX_DEPTH} levels"),
        }
    }
}

impl std::error::Error for WireError {}

/// What the page sends.
#[derive(Debug, Clone, PartialEq)]
pub enum FromPage {
    Ready {
        ids: Vec<String>,
    },
    Event {
        node: u64,
        name: String,
        seq: u64,
        event: Value,
    },
    Call {
        id: u64,
        name: String,
        args: Vec<Value>,
    },
    Callback {
        id: u64,
        args: Vec<Value>,
    },
    Error {
        message: String,
    },
}

pub fn decode(text: &str) -> Result<FromPage, WireError> {
    let json: Json = serde_json::from_str(text).map_err(|error| {
        let said = error.to_string();
        match said.contains("recursion limit") {
            true => WireError::TooDeep,
            false => WireError::Malformed(said),
        }
    })?;
    let object = json
        .as_object()
        .ok_or_else(|| WireError::Malformed("not an object".to_string()))?;
    match text_of(object, "t")? {
        "ready" => Ok(FromPage::Ready {
            ids: field(object, "ids")?
                .as_array()
                .ok_or_else(|| WireError::Malformed("`ids` is not a list".to_string()))?
                .iter()
                .map(|id| {
                    id.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| WireError::Malformed("an id is not a string".to_string()))
                })
                .collect::<Result<_, _>>()?,
        }),
        "event" => Ok(FromPage::Event {
            node: number_of(object, "node")?,
            name: text_of(object, "name")?.to_string(),
            // Absent for an event that is not a field's input.
            seq: object.get("seq").and_then(Json::as_u64).unwrap_or(0),
            event: json_to_value(field(object, "event")?, 0)?,
        }),
        "call" => Ok(FromPage::Call {
            id: number_of(object, "id")?,
            name: text_of(object, "name")?.to_string(),
            args: arguments_of(object)?,
        }),
        "callback" => Ok(FromPage::Callback {
            id: number_of(object, "id")?,
            args: arguments_of(object)?,
        }),
        "error" => Ok(FromPage::Error {
            message: text_of(object, "message")?.to_string(),
        }),
        other => Err(WireError::UnknownTag(other.to_string())),
    }
}

fn field<'a>(object: &'a Map<String, Json>, name: &str) -> Result<&'a Json, WireError> {
    object
        .get(name)
        .ok_or_else(|| WireError::Malformed(format!("`{name}` is missing")))
}

fn text_of<'a>(object: &'a Map<String, Json>, name: &str) -> Result<&'a str, WireError> {
    field(object, name)?
        .as_str()
        .ok_or_else(|| WireError::Malformed(format!("`{name}` is not a string")))
}

fn number_of(object: &Map<String, Json>, name: &str) -> Result<u64, WireError> {
    field(object, name)?
        .as_u64()
        .ok_or_else(|| WireError::Malformed(format!("`{name}` is not a whole number")))
}

fn arguments_of(object: &Map<String, Json>) -> Result<Vec<Value>, WireError> {
    field(object, "args")?
        .as_array()
        .ok_or_else(|| WireError::Malformed("`args` is not a list".to_string()))?
        .iter()
        .map(|each| json_to_value(each, 0))
        .collect()
}

pub fn json_to_value(json: &Json, depth: usize) -> Result<Value, WireError> {
    if depth > MAX_DEPTH {
        return Err(WireError::TooDeep);
    }
    Ok(match json {
        Json::Null => Value::Null,
        Json::Bool(whether) => Value::Bool(*whether),
        Json::Number(number) => Value::Number(number.as_f64().ok_or(WireError::NotANumber)?),
        Json::String(text) => Value::String(text.clone()),
        Json::Array(items) => Value::List(
            items
                .iter()
                .map(|each| json_to_value(each, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        Json::Object(fields) => Value::Record(
            fields
                .iter()
                .map(|(name, each)| Ok((name.clone(), json_to_value(each, depth + 1)?)))
                .collect::<Result<_, WireError>>()?,
        ),
    })
}

pub fn value_to_json(value: &Value, depth: usize) -> Result<Json, WireError> {
    if depth > MAX_DEPTH {
        return Err(WireError::TooDeep);
    }
    Ok(match value {
        Value::Null => Json::Null,
        Value::Bool(whether) => Json::Bool(*whether),
        Value::Number(number) if number.is_finite() => json!(number),
        Value::Number(_) => return Err(WireError::NotANumber),
        Value::String(text) => Json::String(text.clone()),
        Value::List(items) => Json::Array(
            items
                .iter()
                .map(|each| value_to_json(each, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        Value::Record(fields) => Json::Object(
            fields
                .iter()
                .map(|(name, each)| Ok((name.clone(), value_to_json(each, depth + 1)?)))
                .collect::<Result<_, WireError>>()?,
        ),
    })
}

/// The answer to a call, by the id the page gave it.
pub fn encode_reply(id: u64, outcome: Result<Value, String>) -> Result<String, WireError> {
    Ok(match outcome {
        Ok(value) => json!({ "t": "reply", "id": id, "ok": value_to_json(&value, 0)? }),
        Err(message) => json!({ "t": "reply", "id": id, "err": message }),
    }
    .to_string())
}
