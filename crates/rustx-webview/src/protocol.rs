//! The language the Rust tree and the page speak.
//!
//! A [`Patch`] cannot cross as it is: its subtrees hold the developer's
//! closures. So a patch is **lowered** to JSON here, and every closure
//! becomes an id kept in a [`Registry`]; the page sends the id back and the
//! closure runs in this process. A node is addressed by the id it was
//! given when it was built, never by its place in the tree: with the page
//! a message away, a click can arrive after a patch has moved the node
//! it was aimed at.
//!
//! Host to page: `{"t":"mount",…}`, `{"t":"patches",…}`, `{"t":"reply",…}`.
//! Page to host: `{"t":"event",…}`, `{"t":"call",…}`, `{"t":"callback",…}`.

use std::collections::HashMap;

use rustx_dom::diff::{Patch, Path};
use rustx_dom::{
    AttributeValue, Callback, Element, Event, ForeignChild, ForeignNode, Handler, TagElement, Value,
};
use serde_json::{Map, Value as Json, json};

/// How deep a value or a tree may nest on the wire. serde_json refuses to
/// parse past 128; staying below it means what we write we can read.
const MAX_DEPTH: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum WireError {
    /// Not JSON, truncated, or a field missing or of the wrong kind.
    Malformed(String),
    /// A `"t"` this side does not know.
    UnknownTag(String),
    /// NaN and infinity: JSON has no way to write them.
    NotANumber,
    /// Nested deeper than [`MAX_DEPTH`].
    TooDeep,
    /// A patch names a place its tree does not have.
    BadPath(Vec<usize>),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "malformed message: {why}"),
            Self::UnknownTag(tag) => write!(f, "unknown message tag `{tag}`"),
            Self::NotANumber => f.write_str("NaN and infinity cannot cross to the page"),
            Self::TooDeep => write!(f, "nested deeper than {MAX_DEPTH} levels"),
            Self::BadPath(path) => write!(f, "no node at {path:?} in the tree"),
        }
    }
}

impl std::error::Error for WireError {}

/// The closures the page may call back, by id.
#[derive(Default)]
pub struct Registry {
    last: u64,
    handlers: HashMap<u64, Vec<(String, Handler)>>,
    callbacks: HashMap<u64, Callback>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    fn next(&mut self) -> u64 {
        self.last += 1;
        self.last
    }

    fn register(&mut self, tag: &TagElement) -> u64 {
        let id = self.next();
        self.handlers.insert(id, tag.handlers.clone());
        id
    }

    fn callback(&mut self, callback: &Callback) -> u64 {
        let id = self.next();
        self.callbacks.insert(id, callback.clone());
        id
    }

    /// Run the handler `name` of node `node` with the event the page read.
    /// `false` when there is no such node or handler any more: the node was
    /// removed while the event was on its way, and that is not an error.
    pub fn dispatch(&self, node: u64, name: &str, event: &Value) -> bool {
        let Some(handler) = self
            .handlers
            .get(&node)
            .and_then(|all| all.iter().find(|(each, _)| each == name))
            .map(|(_, handler)| handler.clone())
        else {
            return false;
        };
        handler.call(Event::from_value(event));
        true
    }

    /// Run the callback a library was given, with what it passed.
    pub fn call_back(&self, id: u64, arguments: &[Value]) -> bool {
        let Some(callback) = self.callbacks.get(&id).cloned() else {
            return false;
        };
        callback.call(arguments);
        true
    }

    /// Let go of a node's handlers.
    pub fn forget(&mut self, node: u64) {
        self.handlers.remove(&node);
    }
}

/// What the page sends.
#[derive(Debug, Clone, PartialEq)]
pub enum FromPage {
    Event {
        node: u64,
        name: String,
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
        "event" => Ok(FromPage::Event {
            node: number_of(object, "node")?,
            name: text_of(object, "name")?.to_string(),
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

/// The whole tree, for the first frame.
pub fn encode_mount(root: &Element, registry: &mut Registry) -> Result<String, WireError> {
    Ok(json!({ "t": "mount", "node": node(root, registry, 0)? }).to_string())
}

/// A batch of patches. `tree` is the **new** tree: a patch that builds
/// something names where to find it there.
pub fn encode_patches(
    batch: &[Patch],
    tree: &Element,
    registry: &mut Registry,
) -> Result<String, WireError> {
    let operations = batch
        .iter()
        .map(|patch| lower(patch, tree, registry))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "t": "patches", "batch": operations }).to_string())
}

/// The answer to a call, by the id the page gave it.
pub fn encode_reply(id: u64, outcome: Result<Value, String>) -> Result<String, WireError> {
    Ok(match outcome {
        Ok(value) => json!({ "t": "reply", "id": id, "ok": value_to_json(&value, 0)? }),
        Err(message) => json!({ "t": "reply", "id": id, "err": message }),
    }
    .to_string())
}

fn lower(patch: &Patch, tree: &Element, registry: &mut Registry) -> Result<Json, WireError> {
    Ok(match patch {
        Patch::SetText { path, text } => json!({ "op": "text", "path": path.0, "text": text }),
        Patch::SetAttribute { path, name, value } => {
            json!({ "op": "attr", "path": path.0, "name": name, "value": attribute(value) })
        }
        Patch::RemoveAttribute { path, name } => {
            json!({ "op": "unattr", "path": path.0, "name": name })
        }
        Patch::Replace { path, source } => json!({
            "op": "replace", "path": path.0, "node": node(at(tree, source)?, registry, 0)?
        }),
        Patch::Insert {
            parent,
            index,
            source,
        } => json!({
            "op": "insert", "parent": parent.0, "index": index,
            "node": node(at(tree, source)?, registry, 0)?
        }),
        Patch::Remove { parent, index } => {
            json!({ "op": "remove", "parent": parent.0, "index": index })
        }
        Patch::Move { parent, from, to } => {
            json!({ "op": "move", "parent": parent.0, "from": from, "to": to })
        }
        // The page re-tags the element with the new id; the old one stops
        // answering once the session retires it.
        Patch::ReplaceHandlers { path, source } => {
            let Element::Tag(tag) = at(tree, source)? else {
                return Err(WireError::BadPath(source.0.clone()));
            };
            let id = registry.register(tag);
            json!({ "op": "handlers", "path": path.0, "node": id, "on": names(tag) })
        }
        Patch::ForeignProps { path, node: held } => {
            json!({ "op": "foreign", "path": path.0, "node": foreign(held, registry, 0)? })
        }
    })
}

/// The subtree at `path`, walked the way `diff` numbers it: a tag's
/// children, or a fragment's.
fn at<'a>(tree: &'a Element, path: &Path) -> Result<&'a Element, WireError> {
    let mut here = tree;
    for &index in &path.0 {
        here = match here {
            Element::Tag(tag) => tag.children.get(index),
            Element::Fragment(children) => children.get(index),
            _ => None,
        }
        .ok_or_else(|| WireError::BadPath(path.0.clone()))?;
    }
    Ok(here)
}

fn attribute(value: &AttributeValue) -> Json {
    match value {
        AttributeValue::Text(text) => json!({ "text": text }),
        AttributeValue::Flag(whether) => json!({ "flag": whether }),
    }
}

fn names(tag: &TagElement) -> Vec<&str> {
    tag.handlers.iter().map(|(name, _)| name.as_str()).collect()
}

fn node(element: &Element, registry: &mut Registry, depth: usize) -> Result<Json, WireError> {
    if depth > MAX_DEPTH {
        return Err(WireError::TooDeep);
    }
    Ok(match element {
        Element::Empty => json!({ "k": "empty" }),
        Element::Text(text) => json!({ "k": "text", "text": text }),
        Element::Fragment(children) => {
            json!({ "k": "frag", "children": nodes(children, registry, depth)? })
        }
        Element::Tag(tag) => {
            let id = registry.register(tag);
            json!({
                "k": "tag", "id": id, "tag": tag.tag,
                "key": tag.key.as_ref().map(|key| key.as_str()),
                "attrs": tag.attributes.iter()
                    .map(|(name, value)| json!([name, attribute(value)]))
                    .collect::<Vec<_>>(),
                "on": names(tag),
                "children": nodes(&tag.children, registry, depth)?,
            })
        }
        Element::Foreign { node: held, id } => {
            json!({ "k": "foreign", "id": id, "node": foreign(held, registry, depth + 1)? })
        }
    })
}

fn nodes(
    children: &[Element],
    registry: &mut Registry,
    depth: usize,
) -> Result<Vec<Json>, WireError> {
    children
        .iter()
        .map(|child| node(child, registry, depth + 1))
        .collect()
}

fn foreign(held: &ForeignNode, registry: &mut Registry, depth: usize) -> Result<Json, WireError> {
    if depth > MAX_DEPTH {
        return Err(WireError::TooDeep);
    }
    let mut props = Map::new();
    for (name, value) in &held.props {
        props.insert(name.clone(), value_to_json(value, 0)?);
    }
    let callbacks: Vec<Json> = held
        .callbacks
        .iter()
        .map(|(name, callback)| json!([name, registry.callback(callback)]))
        .collect();
    let mut children = Vec::with_capacity(held.children.len());
    for child in &held.children {
        children.push(match child {
            ForeignChild::Node(inner) => {
                json!({ "k": "node", "node": foreign(inner, registry, depth + 1)? })
            }
            ForeignChild::Text(text) => json!({ "k": "text", "text": text }),
            ForeignChild::Value(value) => json!({ "k": "value", "value": value_to_json(value, 0)? }),
            ForeignChild::Element(inner) => {
                json!({ "k": "element", "element": node(inner, registry, depth + 1)? })
            }
        });
    }
    Ok(json!({
        "component": held.component, "props": props,
        "callbacks": callbacks, "children": children,
    }))
}
