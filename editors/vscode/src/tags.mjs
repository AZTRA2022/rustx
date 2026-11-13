// What a `>` just finished, if it finished an opening tag.
//
// Plain JavaScript rather than TypeScript, so the rule can be tested
// without a build and without VS Code: everything here is a function of
// the text in front of the cursor.


/** The characters a name may be made of, after its first. */
const WITHIN = /^[A-Za-z0-9_$.:-]$/;

/** The characters a name may begin with. */
const FIRST = /^[A-Za-z_$]$/;

/**
 * What to insert after the `>` at the end of `before`, or `undefined`
 * when that `>` did not finish an opening tag.
 *
 * @param {string} before every character up to and including the `>`
 * @returns {string | undefined} `"</Card>"`, `"</>"`, or nothing
 */
export function closingFor(before) {
  if (!before.endsWith(">")) {
    return undefined;
  }
  const end = before.length - 1;

  // `<br />` closed itself, and `=>` is a closure.
  const previous = before[end - 1];
  if (previous === "/" || previous === "=" || previous === ">") {
    return undefined;
  }

  const open = openingBefore(before, end);
  if (open === undefined) {
    return undefined;
  }

  const inside = before.slice(open + 1, end);
  if (inside === "") {
    // `<>`, which is a fragment and closes with one.
    return "</>";
  }

  const name = nameAt(inside);
  if (name === undefined) {
    return undefined;
  }
  // `<Card className="x"` is a tag; `< b ` is a comparison.
  const rest = inside.slice(name.length);
  if (rest !== "" && !/^[\s/]/.test(rest)) {
    return undefined;
  }
  if (rest.trimEnd().endsWith("/")) {
    return undefined;
  }
  return `</${name}>`;
}

/**
 * The `<` this `>` closes, or `undefined` when there is none.
 *
 * @param {string} text
 * @param {number} end the offset of the `>`
 * @returns {number | undefined}
 */
function openingBefore(text, end) {
  let at = end - 1;
  while (at >= 0) {
    const letter = text[at];
    // An attribute's expression and an attribute's string are both
    // inside the tag, and both may hold a `>` of their own. They are
    // stepped over whole rather than read.
    if (letter === "}") {
      const open = matching(text, at, "{", "}");
      if (open === undefined) {
        return undefined;
      }
      at = open - 1;
      continue;
    }
    if (letter === '"') {
      const open = text.lastIndexOf('"', at - 1);
      if (open === -1) {
        return undefined;
      }
      at = open - 1;
      continue;
    }
    // A `>` means this one belongs to something else: the tag before it
    // already closed. An unmatched `{` means the cursor is inside an
    // expression, where a `>` is a comparison.
    if (letter === ">" || letter === "{") {
      return undefined;
    }
    if (letter !== "<") {
      at -= 1;
      continue;
    }
    const glued = text[at - 1];
    // `Vec<`, `::<`, `<<` and `a<` are not the start of a tag.
    if (glued !== undefined && (WITHIN.test(glued) || glued === "<")) {
      return undefined;
    }
    const next = text[at + 1];
    // `</` is a closing tag, `<!` a comment or a doctype.
    if (next === "/" || next === "!" || next === "<") {
      return undefined;
    }
    return at;
  }
  return undefined;
}

/**
 * The opener matching the closer at `close`, scanning backwards.
 *
 * @param {string} text
 * @param {number} close
 * @param {string} opener
 * @param {string} closer
 * @returns {number | undefined}
 */
function matching(text, close, opener, closer) {
  let depth = 0;
  for (let at = close; at >= 0; at -= 1) {
    if (text[at] === closer) {
      depth += 1;
    } else if (text[at] === opener) {
      depth -= 1;
      if (depth === 0) {
        return at;
      }
    }
  }
  return undefined;
}

/**
 * The name at the start of a tag's text, if it starts with one.
 *
 * @param {string} inside what sits between the `<` and the `>`
 * @returns {string | undefined}
 */
function nameAt(inside) {
  if (!FIRST.test(inside[0])) {
    return undefined;
  }
  let end = 1;
  while (end < inside.length && WITHIN.test(inside[end])) {
    end += 1;
  }
  const name = inside.slice(0, end);
  // A name that ends on its punctuation was not finished.
  return /[.:-]$/.test(name) ? undefined : name;
}
