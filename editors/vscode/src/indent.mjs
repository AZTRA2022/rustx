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
function codeOf(line, inBlockComment, inString = null) {
  const out = line.split("");
  let at = 0;
  let commented = inBlockComment;
  let quoting = inString;

  // A string that was still open at the end of the line above. The rest
  // of it is not code: a `\` at a line's end continues a literal, and so
  // does a line break inside one that was never closed.
  if (quoting !== null) {
    while (at < out.length) {
      if (line[at] === "\\") {
        out[at] = " ";
        if (at + 1 < out.length) {
          out[at + 1] = " ";
        }
        at += 2;
        continue;
      }
      const closing = line[at] === quoting;
      out[at] = " ";
      at += 1;
      if (closing) {
        quoting = null;
        break;
      }
    }
  }

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
      // Open until proved otherwise: a literal that reaches the end of
      // the line without its closing quote carries on to the next one,
      // and the lines it covers are its own text rather than code.
      quoting = here;
      while (at < out.length) {
        if (line[at] === "\\") {
          out[at] = " ";
          if (at + 1 < out.length) {
            out[at + 1] = " ";
          }
          at += 2;
          continue;
        }
        const closing = line[at] === here;
        out[at] = " ";
        at += 1;
        if (closing) {
          quoting = null;
          break;
        }
      }
      continue;
    }

    at += 1;
  }

  // A `'` is a lifetime far more often than it is an unterminated
  // character literal, and a lifetime must not swallow the rest of the
  // file. Only a `"` is believed when it says it continues.
  if (quoting === "'") {
    quoting = null;
  }

  return { code: out.join(""), commented, quoting };
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
  if (before === "<") {
    return false;
  }
  // A `>` in front of a `<` is `>>` or `=>` in Rust, and in the children
  // of an element it is the tag before this one:
  // `<Card><CardContent>` is how anybody writes it. Refusing it
  // everywhere meant `<CardContent>` opened nothing while
  // `</CardContent>` closed something, so the line came out a frame
  // short and the `</Card>` after it closed the enclosing block.
  if (!inChildren && before === ">") {
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
 *
 * A boolean operator is the same thing: a condition too long for one
 * line is broken before its `&&` or its `||`, and rustfmt writes the
 * rest one step in. Read as statements of their own, those lines came
 * back flush against the line that opened them — which is how
 * `|| entry.name.contains(&needle)` ended up level with the `if` above
 * it in half the pages of this repository.
 *
 * A single `|` is not one: that is a match arm's pattern, and it belongs
 * at the arm's own level.
 */
function isAContinuation(line) {
  return /^\s*([.?]|&&|\|\||==|!=)/.test(line);
}

/**
 * Does this line's code stop in the middle of something?
 *
 * A line ending in `=` has not said what the thing is yet, so the line
 * below it is the rest of the same statement and is written one step in:
 *
 *     if let Ok(Some(output)) =
 *         spawn_blocking(|| run(script)).await
 *     {
 *
 * Only `=` itself, and not `==`, `!=`, `<=`, `>=` or `=>`: those end a
 * line that is complete enough, and `=>` ends a match arm whose body is
 * on the next line at the arm's own level.
 */
function endsUnfinished(code, line) {
  const bare = line.trimEnd();
  if (!bare.endsWith("=")) {
    return false;
  }
  // And it has to be code: `id="waiting"` is blanked to `id=` by
  // `codeOf`, and a line that merely *looks* as though it stopped at an
  // equals sign is every attribute in every `.rsx` there is.
  if (code[bare.length - 1] !== "=") {
    return false;
  }
  return !/[=!<>+\-*/%&|^]=$/.test(bare);
}

/** As deep as indentation is ever allowed to go. */
const MOST = 64;

/** Which frame each closing delimiter is allowed to close. */
const CLOSES = { "}": "{", ")": "(", "]": "[" };

/**
 * Where the `>` that ends the tag opening at `at` is, or `-1`.
 *
 * Not `indexOf(">")`: an attribute holds an expression, and an
 * expression holds comparisons. `<button disabled={n + 1 >= all.len()}>`
 * ends at its last `>` and not at the one inside the braces — and a tag
 * whose end was found inside its own attribute had everything after
 * that point read as though it were outside the tag, which is how the
 * children of such a button lost their level.
 */
function endOfTag(code, at) {
  let braces = 0;
  for (let look = at + 1; look < code.length; look += 1) {
    const here = code[look];
    if (here === "{") {
      braces += 1;
    } else if (here === "}") {
      braces = Math.max(0, braces - 1);
    } else if (here === ">" && braces === 0) {
      return look;
    } else if (here === "<" && braces === 0) {
      // Another tag began before this one was finished: this one has its
      // attributes on the lines below.
      return -1;
    }
  }
  return -1;
}

/**
 * One line, placed, and applied to the list of things still open.
 * finds it.
 */
function placeLine(open, levelsOpen, counts, stamp, code, held, bonus) {
  // Which frames this line opened is told by the stamp they carry, and
  // not by walking the stack to clear a flag: a file with twenty
  // thousand unclosed braces in it has a stack that deep on every line
  // below them, and clearing a flag on each is the whole file squared.
  const isFresh = (frame) => frame !== undefined && frame.stamp === stamp;
  // The innermost frame that is a level. Every bracket is a frame, so
  // that a close matches an open one for one, but only some of them
  // move the text.
  // `levelsOpen` is the frames that are levels, innermost last, kept as
  // a stack beside `open` rather than searched for. Walking `open` to
  // find the innermost level, or filtering it to count them, is one
  // pass over the whole stack per line -- and a file with twenty
  // thousand unclosed braces in it then costs the file squared. That is
  // what made 20,000 lines take five seconds instead of a tenth of one.
  const levelNow = () => {
    const inner = levelsOpen[levelsOpen.length - 1];
    return inner === undefined ? 0 : inner.openedAt + 1;
  };

  let printAt = null;
  const place = () => {
    if (printAt === null) {
      printAt = held === null ? levelNow() + bonus : held;
    }
  };

  // How many frames this line has opened and not closed again. Of
  // those, only the first is a level: `task(move |alive| {` opens a
  // parenthesis and a brace, and Rust is written with the body one step
  // in, not two.
  let opened = 0;
  const push = (kind, openedAt = null) => {
    place();
    opened += 1;
    counts[kind] = (counts[kind] ?? 0) + 1;
    const frame = {
      kind,
      level: opened === 1,
      openedAt: openedAt === null ? printAt : openedAt,
      stamp,
    };
    open.push(frame);
    if (frame.level) {
      levelsOpen.push(frame);
    }
  };
  const pop = () => {
    const frame = open.pop();
    if (frame !== undefined) {
      if (frame.level && levelsOpen[levelsOpen.length - 1] === frame) {
        levelsOpen.pop();
      }
      counts[frame.kind] = Math.max(0, (counts[frame.kind] ?? 0) - 1);
      if (isFresh(frame)) {
        opened = Math.max(0, opened - 1);
      }
    }
    return frame;
  };
  // `kind` is what this delimiter is allowed to close. A `}` closes a
  // `{`, a `</div>` closes a tag, and neither closes the other: a
  // closing tag that popped a block used to shift the whole rest of the
  // file one level out, and a half-typed file is full of chances to do
  // that. Frames in between are discarded with it, because something
  // that was never closed is not going to be.
  const close = (kind) => {
    // `counts` rather than a search: a stack twenty thousand deep must
    // not be walked once per delimiter.
    if (kind !== undefined && (counts[kind] ?? 0) === 0) {
      return undefined;
    }
    if (kind !== undefined) {
      while (open.length > 0 && open[open.length - 1].kind !== kind) {
        pop();
      }
    }
    const frame = pop();
    // A frame this line opened *and* closed is not one of the frames
    // this line opened; `pop` has already taken it off the count. That
    // was the bug: in `match total(&rows) {` the parentheses open and
    // close before the brace, so the brace was the third frame by the
    // old count and was therefore denied its level — and every arm of
    // that `match`, and the body of every `if let Some(v) = f(g(x)) {`,
    // came back flush with the line above.
    //
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
      close(CLOSES[here]);
      continue;
    }

    // The `/>` that ends an attribute list written across lines. It
    // belongs where the tag's name is — which is what every JSX
    // formatter does, and how every page here is written by hand — so
    // the `/` must not place the line: the `>` just below closes the
    // attributes and takes the level of the line that opened them, the
    // same way a `}` takes the level of its `{`. Placing on the `/`
    // left it among the attributes, one step too deep.
    if (
      here === "/" &&
      code.startsWith("/>", at) &&
      (counts.attributes ?? 0) > 0
    ) {
      continue;
    }

    if (here === "<") {
      if (code.startsWith("</", at)) {
        close("tag");
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
      const end = endOfTag(code, at);
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

    if (here === ">" && (counts.attributes ?? 0) > 0) {
      // The `>` that finishes an attribute list written across lines.
      // It closes the attributes and, unless the tag closed itself,
      // opens the element — so `className="p-8">` stays among the
      // attributes, while `}}>` and a `>` alone on its line come back
      // out to the tag's own level.
      const closed = code[at - 1] === "/";
      // The element takes the level of the line that opened the tag,
      // not of the attributes: `<section` is where its children are
      // measured from, however many lines its attributes took.
      const attributes = close("attributes");
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
  // The subset of `open` that moves the text, innermost last.
  const levelsOpen = [];
  // How many frames of each kind are open, so that asking whether a
  // delimiter has anything to close is a lookup rather than a walk.
  const counts = Object.create(null);
  let stamp = 0;
  let commented = false;
  let quoting = null;
  // What a run of continuation lines has settled on, and whether the
  // line before this one opened a level. Both belong to the text rather
  // than to one line, which is why they are kept out here.
  const chain = {
    depth: -1,
    level: null,
    opened_a_level: false,
    unfinished: false,
  };
  const levels = () => levelsOpen.length;

  for (const line of (windows ? text.replace(/\r\n/g, "\n") : text).split("\n")) {
    const wasCommented = commented;
    const wasQuoting = quoting;
    const {
      code,
      commented: stillCommented,
      quoting: stillQuoting,
    } = codeOf(line, commented, quoting);
    commented = stillCommented;
    quoting = stillQuoting;

    const bare = line.trim();
    if (bare === "") {
      out.push("");
      continue;
    }

    // Inside a block comment, the line is left exactly as it was: a
    // comment can hold a drawing, a table, a quoted snippet, and moving
    // its spaces loses something that cannot be put back.
    //
    // And inside a string literal, for a stronger reason: the spaces at
    // the front of the second line of a literal are part of what the
    // program prints.
    if (wasCommented || wasQuoting !== null) {
      out.push(line);
      continue;
    }

    // A chain, a broken condition, or the rest of a statement whose
    // line ended in `=`.
    const continues = isAContinuation(line) || chain.unfinished;
    // The run this line belongs to is the one at the same bracket
    // depth: `.iter()` and `.collect()` either side of a closure body
    // are the same chain, and the `&& …` inside that body is not.
    const together = continues && chain.depth === open.length && chain.level !== null;
    // Written one step past the statement it continues — unless the
    // line above already opened a level, in which case that level *is*
    // the step, and adding another put every `.iter()` under a `{cells`
    // one place too far right.
    const bonus = continues && !chain.opened_a_level ? 1 : 0;
    const before = levels();
    const brackets = open.length;
    stamp += 1;
    const at = placeLine(open, levelsOpen, counts, stamp, code, together ? chain.level : null, bonus);
    if (continues && !together) {
      chain.depth = brackets;
      chain.level = at;
    }
    chain.opened_a_level = levels() > before;
    chain.unfinished = endsUnfinished(code, line);
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
