//! A path, and which screen it names.
//!
//! The table comes from the compiler, which read it off the directory
//! the pages live in. Matching it is pure: a path in, a route and its
//! captured values out, no state and no engine — so the rule that
//! `/device/new` beats `/device/[id]` can be argued about in a test
//! rather than in a window.
//!
//! There is no URL here, and no `location`. A rustx application loads
//! one local page and keeps it: a route is a value, and navigating is
//! changing it (see `rustx_engine::router`).

use crate::Element;

/// What a dynamic segment captured, and what the query string said.
///
/// The two are kept apart on purpose. A segment is part of the route —
/// `/device/[id]` has an `id` and always will — while a query key is
/// part of the *request*: it may be there or not, and no route declares
/// it. A page that mixed them would read `id` and not know which of the
/// two it got; `/device/7?id=9` is the path that proves it matters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Params {
    captured: Vec<(String, String)>,
    query: Vec<(String, String)>,
}

impl Params {
    /// What the query string said under `name`, or `""`.
    ///
    /// `""` for a key nobody wrote and for `?brut` alike — [`has`]
    /// tells those two apart.
    ///
    /// [`has`]: Params::has
    pub fn query(&self, name: &str) -> &str {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
            .unwrap_or_default()
    }

    /// Whether the query string had this key at all.
    pub fn has(&self, name: &str) -> bool {
        self.query.iter().any(|(key, _)| key == name)
    }

    /// Every key the query string had, in the order it was written.
    pub fn queries(&self) -> &[(String, String)] {
        &self.query
    }
}

impl Params {
    /// The value of `name`, or `""` if the path never had one.
    ///
    /// Empty rather than a panic: the caller is generated code, and a
    /// page asking for a name the pattern does not declare is a mistake
    /// the developer can see on screen — not a reason to take the window
    /// down.
    pub fn get(&self, name: &str) -> &str {
        self.captured
            .iter()
            .find(|(captured, _)| captured == name)
            .map(|(_, value)| value.as_str())
            .unwrap_or_default()
    }
}

/// One screen: the path that names it, how to draw it, and the layouts
/// it sits inside.
///
/// Function pointers rather than closures, so the whole table is a
/// `static` the compiler can write out and nobody has to build at
/// start-up.
pub struct Route {
    /// `/device/[id]`, exactly as the directory spelled it.
    pub pattern: &'static str,
    pub render: fn(&Params) -> Element,
    /// Outermost first. Each takes the params too: a layout of
    /// `[proprietaire]/[nom]` is the frame of one repository, and
    /// reading the path back by hand to learn which was the one thing
    /// it could do before.
    pub layouts: &'static [fn(Element, &Params) -> Element],
    /// The screen shown while the page has nothing to show — the
    /// `loading.rsx` nearest to it, if any.
    ///
    /// A page that waits for a `.await` renders `Element::Empty` until
    /// the answer lands. That is correct and it is also a blank screen:
    /// this is what goes there instead.
    pub loading: Option<fn() -> Element>,
    /// The screen shown when drawing the page fell over — the
    /// `error.rsx` nearest to it, if any.
    ///
    /// It takes what went wrong, because a frontier that says "something
    /// happened" is a frontier nobody can act on.
    pub failed: Option<fn(String) -> Element>,
}

/// The route `path` names, and what its dynamic segments captured.
///
/// A static segment beats a parameter, whatever order the table is in:
/// `/device/new` is the "new device" screen even though `/device/[id]`
/// would also have taken it.
pub fn match_route<'a>(table: &'a [Route], path: &str) -> Option<(&'a Route, Params)> {
    let (path, query) = split_query(path);
    let wanted: Vec<&str> = segments(path).collect();
    let query = parsed_query(query);

    let mut best: Option<(&Route, Params, (usize, usize, usize))> = None;
    for route in table {
        let Some(params) = captures(route.pattern, &wanted) else {
            continue;
        };
        let how_well = precision(route.pattern);
        if best.as_ref().is_none_or(|(_, _, best)| how_well > *best) {
            best = Some((route, params, how_well));
        }
    }
    best.map(|(route, mut params, _)| {
        params.query = query;
        (route, params)
    })
}

/// The path, and what followed a `?` — the fragment dropped.
///
/// A fragment is the window's own business: a rustx application loads
/// one local document and `#/route` is how a second window is told
/// where to open. Neither belongs to the path being matched, and both
/// used to be part of the last segment, so `/depots?q=rust` matched
/// nothing at all.
pub fn split_query(path: &str) -> (&str, &str) {
    let path = path.split('#').next().unwrap_or(path);
    match path.split_once('?') {
        Some((path, query)) => (path, query),
        None => (path, ""),
    }
}

/// What a query string says under `name`, or `""`.
///
/// Public because the router answers this question about the path the
/// application is on, and because a query key belongs to nobody's
/// route: it is read by whoever wants it.
pub fn query_of(query: &str, name: &str) -> String {
    parsed_query(query)
        .into_iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .unwrap_or_default()
}

/// Whether a query string holds this key at all.
pub fn has_query(query: &str, name: &str) -> bool {
    parsed_query(query).iter().any(|(key, _)| key == name)
}

/// The same path with `name` set to `value`, or without it when `value`
/// is `None`.
///
/// The order of the other keys is kept, and a key that was already
/// there is replaced where it was rather than moved to the end: a URL
/// that reshuffles itself every time a filter changes is a URL nobody
/// can read.
pub fn with_query(path: &str, name: &str, value: Option<&str>) -> String {
    let (before, query) = split_query(path);
    let mut pairs = parsed_query(query);
    match value {
        Some(value) => match pairs.iter_mut().find(|(key, _)| key == name) {
            Some((_, held)) => *held = value.to_string(),
            None => pairs.push((name.to_string(), value.to_string())),
        },
        None => pairs.retain(|(key, _)| key != name),
    }
    if pairs.is_empty() {
        return before.to_string();
    }
    let written: Vec<String> = pairs
        .iter()
        .map(|(key, value)| {
            if value.is_empty() {
                encoded(key)
            } else {
                format!("{}={}", encoded(key), encoded(value))
            }
        })
        .collect();
    format!("{before}?{}", written.join("&"))
}

/// What may be written in a query string as it is, and what has to be
/// escaped.
///
/// The unreserved set of RFC 3986 plus nothing: an escape too many is
/// read back as the same text, an escape too few is a URL that means
/// something else.
fn encoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            byte => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// `q=rust&page=2` → the pairs, decoded.
fn parsed_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((key, value)) => (decoded(key), decoded(value)),
            // `?brut`: a key that is there and says nothing. Kept,
            // because "is it there" is a question a page asks.
            None => (decoded(pair), String::new()),
        })
        .collect()
}

/// What a browser wrote, read back: `%20` is a space, and so is `+`.
///
/// `+` for a space is only true inside a query string — which is the
/// only place this is used — and it is what every form and every
/// browser writes.
fn decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let digits = &text[index + 1..index + 3];
                match u8::from_str_radix(digits, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    // Not an escape after all: a literal `%`, which a
                    // hand-written URL has every right to hold.
                    Err(_) => {
                        out.push(b'%');
                        index += 1;
                    }
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    // Lossy: what came in was text, and a half-written escape must not
    // be able to take a window down.
    String::from_utf8_lossy(&out).to_string()
}

/// How precisely a pattern describes a path, when several describe it.
///
/// The order is Next.js's own: a static segment beats a named one, a
/// named one beats an attrape-tout, and between two attrape-tout the
/// one with more known segments in front wins. Written as a tuple
/// because that is exactly a comparison in that order.
fn precision(pattern: &str) -> (usize, usize, usize) {
    let mut statics = 0;
    let mut named = 0;
    let mut catching = false;
    for segment in segments(pattern) {
        match kind_of(segment) {
            Kind::Static => statics += 1,
            Kind::Named(_) => named += 1,
            Kind::Rest { .. } => catching = true,
        }
    }
    // A route that catches everything is the last resort, whatever it
    // has in front: `/fichiers/[id]` must win over
    // `/fichiers/[...chemin]`, and both over nothing.
    (statics, named, usize::from(!catching))
}

/// What one segment of a pattern is.
enum Kind<'a> {
    Static,
    Named(&'a str),
    /// `[...name]`, and `[[...name]]` when it may take nothing at all.
    Rest { name: &'a str, optional: bool },
}

fn kind_of(segment: &str) -> Kind<'_> {
    // `[[...name]]` first: it ends in `]]`, so reading the outer
    // brackets off a `[...name]` would see the inner ones as a name.
    if let Some(inner) = segment
        .strip_prefix("[[...")
        .and_then(|rest| rest.strip_suffix("]]"))
        .filter(|name| !name.is_empty())
    {
        return Kind::Rest {
            name: inner,
            optional: true,
        };
    }
    if let Some(inner) = segment
        .strip_prefix("[...")
        .and_then(|rest| rest.strip_suffix(']'))
        .filter(|name| !name.is_empty())
    {
        return Kind::Rest {
            name: inner,
            optional: false,
        };
    }
    match parameter_name(segment) {
        Some(name) => Kind::Named(name),
        None => Kind::Static,
    }
}

/// What `pattern` captures from these segments, or `None` if it does not
/// describe them at all.
///
/// An attrape-tout takes everything that is left, so it may only be the
/// last segment — which is also the only place a folder can put it.
fn captures(pattern: &str, wanted: &[&str]) -> Option<Params> {
    let pattern: Vec<&str> = segments(pattern).collect();

    let mut captured = Vec::new();
    for (at, declared) in pattern.iter().enumerate() {
        match kind_of(declared) {
            Kind::Rest { name, optional } => {
                let rest = wanted.get(at..).unwrap_or_default();
                if rest.is_empty() && !optional {
                    // `/fichiers` is not `/fichiers/[...chemin]`: an
                    // attrape-tout takes the rest, and there has to be
                    // a rest.
                    return None;
                }
                captured.push((name.to_string(), rest.join("/")));
                return Some(Params {
                    captured,
                    query: Vec::new(),
                });
            }
            _ => {
                let Some(given) = wanted.get(at) else {
                    return None;
                };
                match kind_of(declared) {
                    Kind::Named(name) => captured.push((name.to_string(), (*given).to_string())),
                    Kind::Static if declared == given => {}
                    _ => return None,
                }
            }
        }
    }
    // Every segment of the pattern is accounted for; the path must not
    // have more.
    if wanted.len() != pattern.len() {
        return None;
    }
    Some(Params {
        captured,
        query: Vec::new(),
    })
}

/// The segments of a path, ignoring the slashes around and between them.
///
/// So `/device`, `device` and `/device/` are one path, and `/` is a path
/// with no segments. An empty segment is dropped rather than captured:
/// `/device//` must not hand a page an empty id.
fn segments(path: &str) -> impl Iterator<Item = &str> {
    path.split('/').filter(|segment| !segment.is_empty())
}

/// `[id]` → `id`.
fn parameter_name(segment: &str) -> Option<&str> {
    segment
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .filter(|name| !name.is_empty())
}
