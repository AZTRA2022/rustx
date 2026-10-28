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
    "onError",
    "onFocus",
    "onInput",
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
    "onPointerDown",
    "onPointerMove",
    "onPointerUp",
    "onScroll",
    "onSubmit",
    "onTouchEnd",
    "onTouchMove",
    "onTouchStart",
    "onTransitionEnd",
    "onWheel",
];

/// Plain attributes. Sorted, so a missing one is easy to place.
const ATTRIBUTES: &[&str] = &[
    "accept",
    "action",
    "alt",
    "autoComplete",
    "autoFocus",
    "autoPlay",
    "checked",
    "className",
    "cols",
    "colSpan",
    "contentEditable",
    "controls",
    "defaultValue",
    "dir",
    "disabled",
    "download",
    "draggable",
    "height",
    "hidden",
    "href",
    "htmlFor",
    "id",
    "lang",
    "loading",
    "max",
    "maxLength",
    "method",
    "min",
    "minLength",
    "multiple",
    "muted",
    "name",
    "pattern",
    "placeholder",
    "poster",
    "readOnly",
    "rel",
    "required",
    "role",
    "rows",
    "rowSpan",
    "selected",
    "size",
    "spellCheck",
    "src",
    "srcSet",
    "step",
    "style",
    "tabIndex",
    "target",
    "title",
    "type",
    "value",
    "width",
    "wrap",
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
        || name.starts_with("data-")
        || name.starts_with("aria-")
}

/// What the DOM calls this attribute.
///
/// The developer writes the React spelling, because that is the spelling
/// they already know; the DOM wants its own. The translation lives here
/// and nowhere else, so that mounting stays a walk over a tree with no
/// opinions of its own.
pub fn dom_name(name: &str) -> &str {
    match name {
        "className" => "class",
        "htmlFor" => "for",
        "defaultValue" => "value",
        other => other,
    }
}

/// The known name closest to `name`, for an error message that helps.
pub fn nearest(name: &str) -> Option<&'static str> {
    EVENTS
        .iter()
        .chain(ATTRIBUTES)
        .map(|known| (distance(name, known), *known))
        // Past a third of the word, a "did you mean" is noise.
        .filter(|(distance, known)| *distance <= (known.len() / 3).max(2))
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, known)| known)
}

/// Levenshtein, iterative, one row at a time.
fn distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut row: Vec<usize> = (0..=right.len()).collect();

    for (i, left_char) in left.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, &right_char) in right.iter().enumerate() {
            let cost = usize::from(left_char != right_char);
            let next = (row[j + 1] + 1).min(row[j] + 1).min(previous + cost);
            previous = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[right.len()]
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
