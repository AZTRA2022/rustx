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

impl Value {
    /// The text, if it is text.
    ///
    /// Why these exist: a callback is handed a `Value`, and reading
    /// anything out of it meant writing `match` on five variants for
    /// every `onValueChange` — or, in practice, giving up. A library
    /// that says "a string" hands a string, and `value.text()` is the
    /// whole of what the page wanted to write.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    /// The number, if it is a number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// The boolean, if it is a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(whether) => Some(*whether),
            _ => None,
        }
    }

    /// The items, if it is a list.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    /// The fields, if it is a record.
    pub fn as_record(&self) -> Option<&[(String, Value)]> {
        match self {
            Self::Record(fields) => Some(fields),
            _ => None,
        }
    }

    /// Whether it is nothing — `null` or `undefined`, which cross as
    /// the same thing.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// One field of a record, or an item of a list by its number
    /// written out — `get("0")` — so that a path can be followed
    /// without knowing which of the two it crosses.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Self::Record(fields) => fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            Self::List(items) => key.parse::<usize>().ok().and_then(|at| items.get(at)),
            _ => None,
        }
    }

    /// Something nested, by the path to it: `at(["data", "0", "nom"])`.
    pub fn at<'a>(&'a self, path: &[&str]) -> Option<&'a Value> {
        let mut here = self;
        for key in path {
            here = here.get(key)?;
        }
        Some(here)
    }

    /// The value as text, whatever it is.
    ///
    /// A number is written out, a boolean is `true` or `false`, nothing
    /// is `""`. What a page puts on screen, and what it would otherwise
    /// write a `match` for.
    pub fn text(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Bool(whether) => whether.to_string(),
            Self::Number(number) => {
                // `7` and not `7.0`: a slider that passes an integer
                // must not read as a decimal on screen.
                if number.fract() == 0.0 && number.is_finite() {
                    format!("{number:.0}")
                } else {
                    number.to_string()
                }
            }
            Self::String(text) => text.clone(),
            Self::List(items) => items
                .iter()
                .map(Self::text)
                .collect::<Vec<String>>()
                .join(","),
            Self::Record(_) => String::new(),
        }
    }

    /// The value as a number, reading a string that holds one.
    ///
    /// A library that passes `"42"` where its documentation says a
    /// number is a library one still has to work with.
    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(*number),
            Self::String(text) => text.trim().parse().ok(),
            Self::Bool(true) => Some(1.0),
            Self::Bool(false) => Some(0.0),
            _ => None,
        }
    }

    /// Whether it means yes, the way JavaScript means it: `false`,
    /// nothing, zero and the empty string are no, everything else is
    /// yes.
    pub fn truth(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(whether) => *whether,
            Self::Number(number) => *number != 0.0 && !number.is_nan(),
            Self::String(text) => !text.is_empty(),
            Self::List(_) | Self::Record(_) => true,
        }
    }
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

/// `"width: 50%"` as the object React takes.
///
/// The DOM's `style` is a string and React's is a mapping, so a tag
/// written inside a library's component has to cross as one. React
/// throws on a string — "The `style` prop expects a mapping from style
/// properties to values, not a string" — and that exception takes the
/// whole boundary with it: a card with three progress bars in it simply
/// never appeared, and nothing in the page was wrong.
///
/// What is not a declaration is left out rather than guessed at, and a
/// custom property keeps its two dashes: React sets those as written,
/// and camel-casing `--brand` would set a property nobody declared.
pub fn style_object(text: &str) -> Value {
    let mut fields = Vec::new();
    for declaration in text.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        let property = property.trim();
        let value = value.trim();
        if property.is_empty() || value.is_empty() {
            continue;
        }
        fields.push((camel(property), Value::String(value.to_string())));
    }
    Value::Record(fields)
}

/// `background-color` → `backgroundColor`, and `--brand` → `--brand`.
fn camel(property: &str) -> String {
    if property.starts_with("--") {
        return property.to_string();
    }
    let mut out = String::with_capacity(property.len());
    let mut upper = false;
    for letter in property.chars() {
        if letter == '-' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(letter.to_uppercase());
            upper = false;
        } else {
            out.push(letter);
        }
    }
    out
}
