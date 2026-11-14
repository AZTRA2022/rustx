// Reindenting a `.rsx`, and nothing else.

/** Is this character part of a word? */
function isWord(letter) {
  return letter !== undefined && /[A-Za-z0-9_]/.test(letter);
}

/**
 * The line, with strings, comments and anything else that only looks
 * like code blanked out, so that reading brackets reads only brackets.
 *
 * Blanked rather than removed: every index still means what it meant.
 */
function codeOf(line, inBlockComment) {
  const out = line.split("");
  let at = 0;
  let commented = inBlockComment;

  while (at < out.length) {
    const here = line[at];

    if (commented) {
      if (line.startsWith("*/", at)) {
        commented = false;
        out[at] = " ";
        out[at + 1] = " ";
        at += 2;
        continue;
      }
      out[at] = " ";
      at += 1;
      continue;
    }

    if (line.startsWith("//", at)) {
      for (let rest = at; rest < out.length; rest += 1) {
        out[rest] = " ";
      }
      break;
    }

    if (line.startsWith("/*", at)) {
      commented = true;
      out[at] = " ";
      out[at + 1] = " ";
      at += 2;
      continue;
    }

    // A `'` is three things in a `.rsx`: a char literal, a lifetime,
    // and an apostrophe in the text of a tag. Only the first can hold a
    // bracket (`'{'`), so only the first is blanked — and taking
    // "Merci d'être passé" for a literal blanked the `</h1>` after it,
    // which pushed everything below one level in.
    const quoted =
      here === '"' || (here === "'" && /^'(\\.|[^'\\])'/.test(line.slice(at)));
    if (quoted) {
      out[at] = " ";
      at += 1;
      while (at < out.length) {
        if (line[at] === "\\") {
          out[at] = " ";
          out[at + 1] = " ";
          at += 2;
          continue;
        }
        const closing = line[at] === here;
        out[at] = " ";
        at += 1;
        if (closing) {
          break;
        }
      }
      continue;
    }

    at += 1;
  }

  return { code: out.join(""), commented };
}

/**
 * Is the `<` at `at` the start of a tag rather than a generic?
 *
 * `inChildren` says whether we are between a tag and its closing tag.
 * There, a `<` is always a tag — `RustX<span>.</span>` is how anybody
 * writes it, and there is no such thing as a generic in the text of an
 * element. In Rust code it is the other way round, and `Vec<u32>` must
 * not open anything.
 */
function opensATag(code, at, inChildren) {
  // `Vec<`, `::<`, `<<` and `>>` are not tags; `<div` and `<Card` are
  // never written against a word or a colon.
  const before = code[at - 1];
  if (!inChildren && (isWord(before) || before === ":")) {
    return false;
  }
  if (before === "<" || before === ">") {
    return false;
  }
  const next = code[at + 1];
  if (next === undefined || next === "<" || next === "=" || next === " ") {
    return false;
  }
  return true;
}

/**
 * Is this line the continuation of the one above rather than something
 * of its own?
 *
 * A method chain is written one step in — `let data = (0..n)`, then
 * `.map(…)`, then `.collect();` — and that step is not a bracket, so
 * nothing left open knows about it. 
 */
function isAContinuation(line) {
  return /^\s*[.?]/.test(line);
}

/** As deep as indentation is ever allowed to go. */
const MOST = 64;

/**
 * One line, placed, and applied to the list of things still open.
 * finds it.
 */
function placeLine(open, code, line) {
  const bonus = isAContinuation(line) ? 1 : 0;
  // The innermost frame that is a level. Every bracket is a frame, so
  // that a close matches an open one for one, but only some of them
  // move the text.
  const levelNow = () => {
    for (let at = open.length - 1; at >= 0; at -= 1) {
      if (open[at].level) {
        return open[at].openedAt + 1;
      }
    }
    return 0;
  };

  let printAt = null;
  const place = () => {
    if (printAt === null) {
      printAt = levelNow() + bonus;
    }
  };

  let opened = 0;
  const push = (kind, openedAt = null) => {
    place();
    // Of the frames one line opens, only the first is a level:
    // `task(move |alive| {` opens a parenthesis and a brace, and Rust
    // is written with the body one step in, not two.
    opened += 1;
    open.push({
      kind,
      level: opened === 1,
      openedAt: openedAt === null ? printAt : openedAt,
    });
  };
  const close = () => {
    const frame = open.pop();
    if (frame !== undefined && frame.level) {
      opened = Math.max(0, opened - 1);
    }
    // A line that begins by closing something is printed where the line
    // that opened it was printed. That is what puts `}` under its `fn`,
    // `</Card>` under its `<Card>`, and the `})` of a `.map(|x| {` back
    // on the chain rather than out at the statement.
    if (printAt === null && frame !== undefined) {
      printAt = frame.openedAt;
    }
    return frame;
  };

  for (let at = 0; at < code.length; at += 1) {
    const here = code[at];

    if (here === " " || here === "\t") {
      continue;
    }

    if (here === "{" || here === "(" || here === "[") {
      push(here);
      continue;
    }
    if (here === "}" || here === ")" || here === "]") {
      close();
      continue;
    }

    if (here === "<") {
      if (code.startsWith("</", at)) {
        close();
        const end = code.indexOf(">", at);
        at = end === -1 ? code.length : end;
        continue;
      }
      const inChildren =
        open.length > 0 && open[open.length - 1].kind === "tag";
      if (!opensATag(code, at, inChildren)) {
        place();
        continue;
      }
      const end = code.indexOf(">", at);
      if (end === -1) {
        // `<section`, with its attributes on the lines below.
        push("attributes");
        continue;
      }
      if (code[end - 1] !== "/") {
        push("tag");
      } else {
        place();
      }
      at = end;
      continue;
    }

    if (here === ">" && open.some((frame) => frame.kind === "attributes")) {
      // The `>` that finishes an attribute list written across lines.
      // It closes the attributes and, unless the tag closed itself,
      // opens the element — so `className="p-8">` stays among the
      // attributes, while `}}>` and a `>` alone on its line come back
      // out to the tag's own level.
      const closed = code[at - 1] === "/";
      // The element takes the level of the line that opened the tag,
      // not of the attributes: `<section` is where its children are
      // measured from, however many lines its attributes took.
      const attributes = close();
      if (!closed) {
        push("tag", attributes === undefined ? null : attributes.openedAt);
      } else {
        place();
      }
      continue;
    }

    place();
  }

  place();
  return printAt;
}

/**
 * The same text, with the spaces at the front of each line rebuilt.
 *
 * `options` is VS Code's own: `{ tabSize, insertSpaces }`.
 */
export function reindent(text, options) {
  // The editor's own numbers, not trusted. `tabSize` arrives from a
  // setting, and a setting can hold anything somebody typed into a
  // `settings.json`: `0`, `-1`, `"four"`, `1e9`. `String.repeat`
  // throws a `RangeError` on a negative or an infinite count, and
  // Format Document then fails with a stack trace from inside the
  // extension instead of formatting the file.
  const width = Number(options?.tabSize);
  const tabSize =
    Number.isFinite(width) && width >= 1 ? Math.min(Math.floor(width), 16) : 4;
  const unit = options?.insertSpaces === false ? "\t" : " ".repeat(tabSize);
  // The editor hands the text over with the document's own endings.
  // Splitting on "\n" and trimming would eat every "\r", and indenting
  // one line would rewrite the whole file as a side effect.
  const windows = text.includes("\r\n");
  const out = [];
  const open = [];
  let commented = false;

  for (const line of (windows ? text.replace(/\r\n/g, "\n") : text).split("\n")) {
    const wasCommented = commented;
    const { code, commented: stillCommented } = codeOf(line, commented);
    commented = stillCommented;

    const bare = line.trim();
    if (bare === "") {
      out.push("");
      continue;
    }

    // Inside a block comment, the line is left exactly as it was: a
    // comment can hold a drawing, a table, a quoted snippet, and moving
    // its spaces loses something that cannot be put back.
    if (wasCommented) {
      out.push(line);
      continue;
    }

    const at = placeLine(open, code, line);
    // And the depth is capped. A file that opens a thousand brackets
    // and closes none is not a file anybody wrote, but it is a file an
    // editor can be handed — half a paste, a generated fixture, a
    // fuzzer — and `"    ".repeat(900000)` is megabytes of spaces per
    // line and an editor that stops responding. Past this depth the
    // indentation means nothing anyway.
    const depth = Number.isFinite(at) ? Math.min(Math.max(0, at), MOST) : 0;
    out.push(unit.repeat(depth) + bare);
  }

  return out.join(windows ? "\r\n" : "\n");
}

/**
 * Is `after` the same source as `before`, give or take the indentation?
 *
 * The check that makes Format Document safe to run on a file somebody
 * has not saved. Whatever the rule above does to the spaces at the
 * front of a line, no character of theirs may appear, vanish, or move
 * to another line — so a bug in the indenter costs a formatting that
 * did nothing, rather than a document with a line missing out of the
 * middle of it.
 *
 * @param {string} before
 * @param {string} after
 * @returns {boolean}
 */
export function onlyIndentationChanged(before, after) {
  if (typeof before !== "string" || typeof after !== "string") {
    return false;
  }
  const bare = (text) =>
    text
      .replace(/\r\n/g, "\n")
      .split("\n")
      .map((line) => line.trim())
      .join("\n");
  return bare(before) === bare(after);
}
