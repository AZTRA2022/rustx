//! What a tag may be given, and what it is called in the DOM.
//!
//! One file, two tables, and the only place to edit when something is
//! missing. The design document promises "every tag and prop"; the way to
//! keep that promise honest is to refuse what is not in the table rather
//! than to drop it quietly. A dropped `classname` is an afternoon spent
//! wondering why the style does not apply.

/// Event handlers, written the way React writes them.
const EVENTS: &[&str] = &[
    "onAnimationEnd",
    "onAnimationIteration",
    "onAnimationStart",
    "onBlur",
    "onChange",
    "onClick",
    "onContextMenu",
    "onCopy",
    "onCut",
    "onDoubleClick",
    "onDragEnd",
    "onDragEnter",
    "onDragLeave",
    "onDragOver",
    "onDragStart",
    "onDrop",
    "onEnded",
    "onError",
    "onFocus",
    "onInput",
    "onInvalid",
    "onKeyDown",
    "onKeyPress",
    "onKeyUp",
    "onLoad",
    "onMouseDown",
    "onMouseEnter",
    "onMouseLeave",
    "onMouseMove",
    "onMouseOut",
    "onMouseOver",
    "onMouseUp",
    "onPaste",
    "onPointerCancel",
    "onPointerDown",
    "onPointerEnter",
    "onPointerLeave",
    "onPointerMove",
    "onPointerUp",
    "onReset",
    "onScroll",
    "onSelect",
    "onSubmit",
    "onTouchEnd",
    "onTouchMove",
    "onTouchStart",
    "onTimeUpdate",
    "onTransitionEnd",
    "onWheel",
];

/// Plain attributes. Sorted, so a missing one is easy to place.
const ATTRIBUTES: &[&str] = &[
    "accept",
    "accessKey",
    "action",
    "alt",
    "autoComplete",
    "autoFocus",
    "autoPlay",
    "checked",
    "cite",
    "crossOrigin",
    "datetime",
    "className",
    "cols",
    "colSpan",
    "contentEditable",
    "controls",
    "defaultValue",
    "dir",
    "disabled",
    "download",
    "enterKeyHint",
    "draggable",
    "height",
    "form",
    "headers",
    "hidden",
    "href",
    "htmlFor",
    "id",
    "inputMode",
    "lang",
    "list",
    "loading",
    "max",
    "maxLength",
    "method",
    "min",
    "minLength",
    "multiple",
    "muted",
    "name",
    "open",
    "pattern",
    "placeholder",
    "playsInline",
    "poster",
    "preload",
    "readOnly",
    "referrerPolicy",
    "rel",
    "reversed",
    "required",
    "role",
    "rows",
    "scope",
    "rowSpan",
    "selected",
    "size",
    "start",
    "spellCheck",
    "src",
    "srcSet",
    "step",
    "style",
    "tabIndex",
    "target",
    "title",
    "translate",
    "type",
    "value",
    "width",
    "wrap",
];

/// SVG attributes. Without these an icon cannot be drawn at all, and an
/// application that cannot draw an icon is not one anybody ships.
///
/// Several are hyphenated in the DOM (`stroke-width`) and camelCase in
/// React (`strokeWidth`); both spellings are accepted and `dom_name`
/// settles which one reaches the element.
const SVG_ATTRIBUTES: &[&str] = &[
    "clipPath",
    "clipRule",
    "cx",
    "cy",
    "d",
    "dx",
    "dy",
    "fill",
    "fillOpacity",
    "fillRule",
    "gradientUnits",
    "offset",
    "opacity",
    "orient",
    "points",
    "preserveAspectRatio",
    "r",
    "rx",
    "ry",
    "stopColor",
    "stopOpacity",
    "stroke",
    "strokeDasharray",
    "strokeDashoffset",
    "strokeLinecap",
    "strokeLinejoin",
    "strokeOpacity",
    "strokeWidth",
    "textAnchor",
    "transform",
    "vectorEffect",
    "viewBox",
    "x",
    "x1",
    "x2",
    "xmlns",
    "y",
    "y1",
    "y2",
];

/// Is this attribute an event handler rather than a value?
pub fn is_event(name: &str) -> bool {
    EVENTS.contains(&name)
}

/// Does the framework know this attribute at all?
///
/// `data-*` and `aria-*` are open by design — they are namespaces the web
/// platform hands to the application, and no table could hold them.
pub fn is_known(name: &str) -> bool {
    is_event(name)
        || ATTRIBUTES.contains(&name)
        || SVG_ATTRIBUTES.contains(&name)
        // SVG's own hyphenated spellings, which the DOM takes verbatim.
        || SVG_ATTRIBUTES.contains(&camel_case(name).as_str())
        || name.starts_with("data-")
        || name.starts_with("aria-")
        || name.starts_with("xlink:")
        || name.starts_with("xml:")
}

/// `stroke-width` → `strokeWidth`, so that both spellings of an SVG
/// attribute are recognised as the same one.
fn camel_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut capitalise = false;
    for character in name.chars() {
        if character == '-' {
            capitalise = true;
        } else if capitalise {
            out.extend(character.to_uppercase());
            capitalise = false;
        } else {
            out.push(character);
        }
    }
    out
}

/// `strokeWidth` → `stroke-width`, the spelling an SVG element wants.
fn hyphenate(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    for character in name.chars() {
        if character.is_ascii_uppercase() {
            out.push('-');
            out.extend(character.to_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}

/// What the DOM calls this attribute.
///
/// The developer writes the React spelling, because that is the spelling
/// they already know; the DOM wants its own. The translation lives here
/// and nowhere else, so that mounting stays a walk over a tree with no
/// opinions of its own.
pub fn dom_name(name: &str) -> std::borrow::Cow<'_, str> {
    match name {
        "className" => "class".into(),
        "htmlFor" => "for".into(),
        "defaultValue" => "value".into(),
        // `viewBox` and `preserveAspectRatio` keep their capitals in SVG;
        // everything else camelCase in that table is hyphenated there.
        "viewBox" | "preserveAspectRatio" | "clipPath" | "gradientUnits" => name.into(),
        other if SVG_ATTRIBUTES.contains(&other) && other.chars().any(char::is_uppercase) => {
            hyphenate(other).into()
        }
        other => other.into(),
    }
}

/// The known name closest to `name`, for an error message that helps.
pub fn nearest(name: &str) -> Option<&'static str> {
    closest(
        name,
        EVENTS.iter().chain(ATTRIBUTES).chain(SVG_ATTRIBUTES).copied(),
    )
}

/// The closest of `known` to `name`, when one is close enough to be
/// worth saying out loud.
///
/// The threshold is the shorter word's length divided by three, which
/// keeps `dvi` → `div` and drops `bar` → `dir`: a suggestion that is not
/// the thing the developer meant costs them more than no suggestion.
pub(crate) fn closest(
    name: &str,
    known: impl Iterator<Item = &'static str>,
) -> Option<&'static str> {
    known
        .map(|known| (distance(name, known), known))
        .filter(|(distance, known)| *distance <= name.len().min(known.len()) / 3)
        .min_by_key(|(distance, known)| (*distance, *known))
        .map(|(_, known)| known)
}

/// Damerau–Levenshtein, counting a swap of two neighbours as one edit.
///
/// Plain Levenshtein charges two for a transposition, which puts `dvi`
/// two edits from `div` — and two edits is where suggestions stop being
/// suggestions. Swapping a pair of letters is the typo people actually
/// make, so it costs one here.
fn distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let mut grid = vec![vec![0_usize; right.len() + 1]; left.len() + 1];

    for (i, row) in grid.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in grid[0].iter_mut().enumerate() {
        *cell = j;
    }

    for i in 1..=left.len() {
        for j in 1..=right.len() {
            let cost = usize::from(left[i - 1] != right[j - 1]);
            let mut best = (grid[i - 1][j] + 1)
                .min(grid[i][j - 1] + 1)
                .min(grid[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && left[i - 1] == right[j - 2] && left[i - 2] == right[j - 1] {
                best = best.min(grid[i - 2][j - 2] + 1);
            }
            grid[i][j] = best;
        }
    }
    grid[left.len()][right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typo_finds_its_neighbour() {
        assert_eq!(nearest("classname"), Some("className"));
        assert_eq!(nearest("onclick"), Some("onClick"));
    }

    #[test]
    fn something_unrelated_suggests_nothing() {
        assert_eq!(nearest("zzzzzzzzzzzz"), None);
    }
}
