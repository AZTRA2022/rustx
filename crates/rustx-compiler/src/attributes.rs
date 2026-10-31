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
    // The rest of what a page of HTML uses. The promise is
    // "every tag and prop", so this table is the whole surface
    // rather than a sample of it; the HTML spelling of each is
    // recognised by `canonical` without being written here.
    "abbr",
    "acceptCharset",
    "allow",
    "allowFullScreen",
    "alt",
    "async",
    "autoCapitalize",
    "autoSave",
    "capture",
    "cellPadding",
    "cellSpacing",
    "charSet",
    "content",
    "coords",
    "dateTime",
    "decoding",
    "default",
    "defer",
    "encType",
    "fetchPriority",
    "formAction",
    "formEncType",
    "formMethod",
    "formNoValidate",
    "formTarget",
    "high",
    "hrefLang",
    "httpEquiv",
    "inert",
    "integrity",
    "is",
    "isMap",
    "itemID",
    "itemProp",
    "itemRef",
    "itemScope",
    "itemType",
    "kind",
    "label",
    "loop",
    "low",
    "media",
    "noModule",
    "noValidate",
    "nonce",
    "optimum",
    "ping",
    "popover",
    "popoverTarget",
    "popoverTargetAction",
    "property",
    "sandbox",
    "shape",
    "sizes",
    "slot",
    "span",
    "srcDoc",
    "srcLang",
    "summary",
    "useMap",
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
    // Presentation attributes: an icon or a chart drawn in SVG
    // uses these the way HTML uses CSS, and both spellings of
    // each are accepted.
    "alignmentBaseline",
    "baselineShift",
    "clipPathUnits",
    "colorInterpolation",
    "colorInterpolationFilters",
    "dominantBaseline",
    "fillRule",
    "filter",
    "filterUnits",
    "floodColor",
    "floodOpacity",
    "fontFamily",
    "fontSize",
    "fontStretch",
    "fontStyle",
    "fontVariant",
    "fontWeight",
    "fr",
    "fx",
    "fy",
    "gradientTransform",
    "imageRendering",
    "letterSpacing",
    "lightingColor",
    "markerEnd",
    "markerHeight",
    "markerMid",
    "markerStart",
    "markerUnits",
    "markerWidth",
    "mask",
    "maskContentUnits",
    "maskUnits",
    "mixBlendMode",
    "paintOrder",
    "pathLength",
    "patternContentUnits",
    "patternTransform",
    "patternUnits",
    "pointerEvents",
    "refX",
    "refY",
    "rotate",
    "shapeRendering",
    "spreadMethod",
    "startOffset",
    "stopColor",
    "strokeMiterlimit",
    "textDecoration",
    "textLength",
    "textRendering",
    "transformOrigin",
    "visibility",
    "wordSpacing",
    "writingMode",
];

/// The attributes whose HTML spelling is not simply the lowercase of
/// the JSX one, written out because nothing can derive them.
const HTML_SPELLINGS: &[(&str, &str)] = &[
    ("class", "className"),
    ("for", "htmlFor"),
    ("http-equiv", "httpEquiv"),
    ("accept-charset", "acceptCharset"),
];

/// Is this attribute an event handler rather than a value?
pub fn is_event(name: &str) -> bool {
    EVENTS.contains(&name)
}

/// The table entry a written name refers to, whichever dialect it was
/// written in.
///
/// A `.rsx` is written by somebody who knows HTML, or JSX, or both, and
/// `class` and `className` are the same attribute. Each table holds one
/// spelling — React's, because that is the one with the capitals a
/// lowercase form can be derived from — and this is what recognises the
/// other: the lowercase of an entry (`tabindex` for `tabIndex`), the
/// hyphenated form of an SVG one (`stroke-width` for `strokeWidth`), and
/// the handful whose HTML spelling is a different word.
pub fn canonical(name: &str) -> Option<&'static str> {
    if let Some(event) = EVENTS.iter().find(|event| **event == name) {
        return Some(event);
    }
    if let Some((_, jsx)) = HTML_SPELLINGS.iter().find(|(html, _)| *html == name) {
        return Some(jsx);
    }
    let lowered = name.to_ascii_lowercase();
    ATTRIBUTES
        .iter()
        .chain(SVG_ATTRIBUTES)
        .find(|known| {
            if **known == name {
                return true;
            }
            // An entry whose HTML spelling is a different word does not
            // also answer to its own lowercase: HTML has `class`, not
            // `classname`, and a dropped `classname` is the afternoon
            // this table exists to save.
            let aliased = HTML_SPELLINGS.iter().any(|(_, jsx)| jsx == *known);
            !aliased && (known.to_ascii_lowercase() == lowered || hyphenate(known) == lowered)
        })
        .copied()
}

/// Does the framework know this attribute at all?
///
/// `data-*` and `aria-*` are open by design — they are namespaces the web
/// platform hands to the application, and no table could hold them.
pub fn is_known(name: &str) -> bool {
    canonical(name).is_some()
        || name.starts_with("data-")
        || name.starts_with("aria-")
        || name.starts_with("xlink:")
        || name.starts_with("xml:")
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
    // Written in HTML already: the DOM's spelling is the one on the
    // page, and there is nothing to translate.
    if !name.chars().any(char::is_uppercase) {
        return name.into();
    }

    match name {
        "className" => "class".into(),
        "htmlFor" => "for".into(),
        "defaultValue" => "value".into(),
        "httpEquiv" => "http-equiv".into(),
        "acceptCharset" => "accept-charset".into(),
        // These keep their capitals in SVG; everything else camelCase in
        // that table is hyphenated there.
        "viewBox"
        | "preserveAspectRatio"
        | "clipPath"
        | "clipPathUnits"
        | "gradientUnits"
        | "gradientTransform"
        | "maskUnits"
        | "maskContentUnits"
        | "patternUnits"
        | "patternContentUnits"
        | "patternTransform"
        | "spreadMethod"
        | "startOffset"
        | "pathLength"
        | "markerWidth"
        | "markerHeight"
        | "markerUnits"
        | "refX"
        | "refY"
        | "baseFrequency"
        | "numOctaves"
        | "stdDeviation"
        | "tableValues"
        | "primitiveUnits"
        | "filterUnits"
        | "surfaceScale"
        | "specularConstant"
        | "specularExponent"
        | "diffuseConstant"
        | "kernelMatrix"
        | "keyPoints"
        | "keySplines"
        | "keyTimes"
        | "repeatCount"
        | "repeatDur"
        | "attributeName"
        | "attributeType"
        | "calcMode"
        | "textLength"
        | "lengthAdjust"
        | "systemLanguage"
        | "requiredFeatures"
        | "requiredExtensions"
        | "xChannelSelector"
        | "yChannelSelector"
        | "zoomAndPan"
        | "edgeMode"
        | "preserveAlpha" => name.into(),
        other if SVG_ATTRIBUTES.contains(&other) => hyphenate(other).into(),
        // Every other camelCase name is an HTML attribute, and HTML
        // attributes are lowercase: `tabIndex` is `tabindex`.
        other => other.to_ascii_lowercase().into(),
    }
}

/// The known name closest to `name`, for an error message that helps.
pub fn nearest(name: &str) -> Option<&'static str> {
    closest(
        name,
        EVENTS
            .iter()
            .chain(ATTRIBUTES)
            .chain(SVG_ATTRIBUTES)
            .copied(),
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

/// `{ backgroundColor: "#09090b", gap: "12px" }` → `background-color: #09090b; gap: 12px`
///
/// The object of styles is the one piece of React's prop dialect that
/// cannot be passed through: the DOM wants a declaration string, and
/// what TSX writes is a JavaScript object. So it is read here — by the
/// same scanning the rest of the compiler does, not by a JSON parser,
/// because what is inside a value is the developer's Rust-free text and
/// may hold commas, brackets and quotes of its own.
///
/// A value has to be a string. React appends `px` to a bare number for
/// some properties and not for others (`width: 10` is pixels,
/// `flexGrow: 10` is not), and a framework that guessed would be wrong
/// in a way nobody could see on screen — so a number is refused by
/// name, with the string to write instead.
///
/// The error is the complaint, ready to be put in front of the
/// developer by the caller that knows where the attribute sits.
pub fn style_string(object: &str) -> Result<String, String> {
    let inside = object
        .trim()
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .ok_or_else(|| "a style has to be an object: style={{ … }}".to_string())?;

    let mut declarations = Vec::new();
    for entry in top_level_commas(inside) {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (name, value) = entry
            .split_once(':')
            .ok_or_else(|| format!("`{entry}` is not a `property: \"value\"` pair"))?;
        let property = css_name(name.trim().trim_matches('"'));
        let value = value.trim();

        let Some(text) = value
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
        else {
            return Err(format!(
                "`{property}` is given `{value}`, and a style value has to be a string — \
                 write \"{value}px\" if you meant pixels. React adds `px` to some \
                 properties and not to others, and rustx will not guess which."
            ));
        };
        declarations.push(format!("{property}: {text}"));
    }
    Ok(declarations.join("; "))
}

/// `backgroundColor` → `background-color`. A name already written in
/// kebab-case passes through, because that is what a quoted key is for.
fn css_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for character in name.chars() {
        if character.is_ascii_uppercase() {
            out.push('-');
            out.push(character.to_ascii_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}

/// The entries of an object, split on the commas that belong to it —
/// not on the ones inside `rgba(255,255,255,0.1)` or inside a string.
fn top_level_commas(inside: &str) -> Vec<&str> {
    let bytes = inside.as_bytes();
    let mut entries = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                index = crate::scan::skip_string(inside, index).unwrap_or(index + 1);
                continue;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                entries.push(&inside[start..index]);
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    entries.push(&inside[start..]);
    entries
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
