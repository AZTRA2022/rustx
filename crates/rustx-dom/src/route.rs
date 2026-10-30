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

/// What a dynamic segment captured.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Params {
    captured: Vec<(String, String)>,
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
    /// Outermost first.
    pub layouts: &'static [fn(Element) -> Element],
}

/// The route `path` names, and what its dynamic segments captured.
///
/// A static segment beats a parameter, whatever order the table is in:
/// `/device/new` is the "new device" screen even though `/device/[id]`
/// would also have taken it.
pub fn match_route<'a>(table: &'a [Route], path: &str) -> Option<(&'a Route, Params)> {
    let wanted: Vec<&str> = segments(path).collect();

    let mut best: Option<(&Route, Params, usize)> = None;
    for route in table {
        let Some(params) = captures(route.pattern, &wanted) else {
            continue;
        };
        let statics = segments(route.pattern)
            .filter(|segment| !is_parameter(segment))
            .count();
        if best.as_ref().is_none_or(|(_, _, best)| statics > *best) {
            best = Some((route, params, statics));
        }
    }
    best.map(|(route, params, _)| (route, params))
}

/// What `pattern` captures from these segments, or `None` if it does not
/// describe them at all.
fn captures(pattern: &str, wanted: &[&str]) -> Option<Params> {
    let pattern: Vec<&str> = segments(pattern).collect();
    if pattern.len() != wanted.len() {
        return None;
    }

    let mut captured = Vec::new();
    for (declared, given) in pattern.iter().zip(wanted) {
        match parameter_name(declared) {
            Some(name) => captured.push((name.to_string(), (*given).to_string())),
            None if declared == given => {}
            None => return None,
        }
    }
    Some(Params { captured })
}

/// The segments of a path, ignoring the slashes around and between them.
///
/// So `/device`, `device` and `/device/` are one path, and `/` is a path
/// with no segments. An empty segment is dropped rather than captured:
/// `/device//` must not hand a page an empty id.
fn segments(path: &str) -> impl Iterator<Item = &str> {
    path.split('/').filter(|segment| !segment.is_empty())
}

fn is_parameter(segment: &str) -> bool {
    parameter_name(segment).is_some()
}

/// `[id]` → `id`.
fn parameter_name(segment: &str) -> Option<&str> {
    segment
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .filter(|name| !name.is_empty())
}
