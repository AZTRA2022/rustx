// The inputs nobody writes on purpose.
//
// Both of these modules run on a keystroke, on text an editor happens
// to be holding: half a paste, a file that is still being typed, a
// fixture a generator produced, a minified blob somebody opened by
// mistake. None of that is valid `.rsx`, and none of it is a reason to
// throw an exception, hang the editor or eat somebody's source.
//
// So this file asserts three things and almost nothing about taste:
//
//   * nothing throws, whatever it is given;
//   * nothing takes long enough to be felt;
//   * `reindent` never changes a character that is not indentation.
//
// The example-by-example tests live in `indent.test.mjs` and
// `tags.test.mjs`. This one is about the floor underneath them.

import { test } from "node:test";
import assert from "node:assert/strict";

import { onlyIndentationChanged, reindent } from "../src/indent.mjs";
import { closingFor } from "../src/tags.mjs";

/** The options VS Code normally passes. */
const NORMAL = { tabSize: 4, insertSpaces: true };

/** Everything but the indentation. */
function bare(text) {
  return text
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((line) => line.trim())
    .join("\n");
}

/** How long something took, in milliseconds. */
function took(work) {
  const started = process.hrtime.bigint();
  work();
  return Number(process.hrtime.bigint() - started) / 1e6;
}

// --------------------------------------------------------------- input

/**
 * The shapes that break things: unbalanced, unterminated, enormous,
 * empty, and not text at all.
 */
const AWKWARD = [
  "",
  "\n",
  "\r\n",
  "\r",
  "\n\n\n\n",
  " ",
  "\t\t\t",
  "{",
  "}",
  "<",
  ">",
  "<<<<<<<",
  ">>>>>>>",
  "{".repeat(5000),
  "}".repeat(5000),
  "<".repeat(5000),
  "(".repeat(5000),
  '"',
  '"'.repeat(999),
  "/*",
  "*/",
  "//",
  "/* /* /* nested and never closed",
  '"a string that never ends',
  "let x = r#\"a raw string\"#;",
  "fn f() { let a: Vec<Vec<Vec<u32>>> = vec![]; }",
  "return <div>",
  "return <div></div>",
  "<>",
  "</>",
  "a < b && c > d",
  "if x < y { return <p>{x}</p>; }",
  // A line of every character that means something to either module.
  "<>{}\"'/*-+=:;.,|&!?#$@\\`~^%",
  // Mixed endings in one file, which is how a merge leaves things.
  "fn a() {\r\n    return <p>hi</p>;\n}\r\n",
  // Not ASCII.
  "let émoji = \"🙂\"; return <p>{émoji}</p>;",
  // A lone surrogate: a real string an editor can hold.
  "\ud800",
  "a\u0000b",
  // One very long line, which is what a minified file is.
  "x".repeat(200000),
  // Deeply nested, which is what makes an indenter allocate.
  Array.from({ length: 2000 }, (_, at) => `${"{".repeat(1)}line ${at}`).join("\n"),
];

/** The option objects an editor should never send, and sometimes does. */
const AWKWARD_OPTIONS = [
  NORMAL,
  { tabSize: 0, insertSpaces: true },
  { tabSize: -1, insertSpaces: true },
  { tabSize: 1e9, insertSpaces: true },
  { tabSize: Infinity, insertSpaces: true },
  { tabSize: -Infinity, insertSpaces: true },
  { tabSize: NaN, insertSpaces: true },
  { tabSize: 2.7, insertSpaces: true },
  { tabSize: "4", insertSpaces: true },
  { tabSize: "four", insertSpaces: true },
  { tabSize: null, insertSpaces: true },
  { tabSize: undefined, insertSpaces: true },
  { tabSize: 4, insertSpaces: false },
  { tabSize: 4 },
  {},
];

// ------------------------------------------------------------- indent

test("reindent_never_throws_whatever_it_is_given", () => {
  for (const text of AWKWARD) {
    for (const options of AWKWARD_OPTIONS) {
      assert.doesNotThrow(
        () => reindent(text, options),
        `threw on ${JSON.stringify(text.slice(0, 40))} with ${JSON.stringify(options)}`,
      );
    }
  }
});

test("reindent_always_hands_back_a_string", () => {
  for (const text of AWKWARD) {
    assert.equal(typeof reindent(text, NORMAL), "string");
  }
});

test("reindent_changes_nothing_but_the_indentation", () => {
  // The invariant the formatter is allowed to rely on. If this ever
  // fails, somebody's file is losing characters on Format Document.
  for (const text of AWKWARD) {
    for (const options of AWKWARD_OPTIONS) {
      const out = reindent(text, options);
      assert.equal(
        bare(out),
        bare(text),
        `content moved on ${JSON.stringify(text.slice(0, 60))}`,
      );
    }
  }
});

test("reindent_keeps_the_line_count", () => {
  // An indenter that adds or drops a line moves every `rustc`
  // diagnostic under it.
  for (const text of AWKWARD) {
    const out = reindent(text, NORMAL);
    assert.equal(
      out.replace(/\r\n/g, "\n").split("\n").length,
      text.replace(/\r\n/g, "\n").split("\n").length,
      `line count moved on ${JSON.stringify(text.slice(0, 60))}`,
    );
  }
});

test("reindent_is_idempotent", () => {
  // Format Document twice is Format Document once. Without this, a
  // formatter on save fights every save.
  for (const text of AWKWARD) {
    const once = reindent(text, NORMAL);
    assert.equal(reindent(once, NORMAL), once, `drifted on ${JSON.stringify(text.slice(0, 60))}`);
  }
});

test("reindent_keeps_the_line_endings_it_was_given", () => {
  const windows = "fn a() {\r\n    return <p>hi</p>;\r\n}\r\n";
  assert.ok(reindent(windows, NORMAL).includes("\r\n"));
  const unix = "fn a() {\n    return <p>hi</p>;\n}\n";
  assert.ok(!reindent(unix, NORMAL).includes("\r"));
});

test("a_pathological_depth_does_not_allocate_without_end", () => {
  // 20,000 unclosed braces once meant 20,000 levels of indentation on
  // every line below them -- megabytes of spaces per line.
  const text = `${"{\n".repeat(20000)}deep`;
  let out;
  const spent = took(() => {
    out = reindent(text, NORMAL);
  });
  assert.ok(out.length < text.length * 300, `grew to ${out.length} from ${text.length}`);
  assert.ok(spent < 4000, `took ${spent}ms`);
});

test("a_large_file_is_indented_in_a_reasonable_time", () => {
  const line = "            <Button variant=\"outline\" onClick={move |_| n += 1}>go</Button>\n";
  const text = `fn P() -> Element {\n    return <div>\n${line.repeat(20000)}    </div>;\n}\n`;
  const spent = took(() => reindent(text, NORMAL));
  assert.ok(spent < 5000, `took ${spent}ms on ${text.length} characters`);
});

// --------------------------------------------------------------- tags

test("closingFor_never_throws_whatever_it_is_given", () => {
  const extra = ["", ">", "<>", "</>", "=>", "->", ">>", "<{}>", '<a b="c>', "<{>"];
  for (const text of [...AWKWARD, ...extra]) {
    assert.doesNotThrow(() => closingFor(text), `threw on ${JSON.stringify(text.slice(0, 40))}`);
    // And on every suffix ending in `>`, which is what the editor
    // actually hands it as somebody types a tag out one key at a time.
    const typed = `${text}>`;
    assert.doesNotThrow(() => closingFor(typed), `threw on ${JSON.stringify(typed.slice(0, 40))}`);
  }
});

test("closingFor_survives_what_is_not_a_string", () => {
  for (const nonsense of [undefined, null, 0, 1, true, {}, [], Symbol("x")]) {
    assert.doesNotThrow(() => closingFor(nonsense), `threw on ${String(nonsense)}`);
    assert.equal(closingFor(nonsense), undefined);
  }
});

test("closingFor_hands_back_a_closing_tag_or_nothing", () => {
  for (const text of AWKWARD) {
    const got = closingFor(`${text}>`);
    if (got === undefined) {
      continue;
    }
    assert.equal(typeof got, "string");
    assert.ok(got.startsWith("</") && got.endsWith(">"), `odd answer ${JSON.stringify(got)}`);
    assert.ok(!got.includes("\n"), `a closing tag is one line: ${JSON.stringify(got)}`);
  }
});

test("closingFor_is_fast_on_a_long_document", () => {
  // Every `>` typed scans what is in front of the cursor. A megabyte
  // of prose in front of it must not be felt.
  const long = `${"word ".repeat(200000)}<div>`;
  const spent = took(() => closingFor(long));
  assert.ok(spent < 50, `took ${spent}ms`);
});

test("closingFor_is_fast_with_many_braces_in_front_of_it", () => {
  // Braces are stepped over by a backwards scan of their own, which is
  // where a quadratic cost would hide.
  const braces = `${"{a}".repeat(100000)}<div>`;
  const spent = took(() => closingFor(braces));
  assert.ok(spent < 50, `took ${spent}ms`);
});

test("closingFor_is_fast_on_an_unterminated_string", () => {
  const quoted = `${'"'.repeat(100000)}<div>`;
  const spent = took(() => closingFor(quoted));
  assert.ok(spent < 50, `took ${spent}ms`);
});

test("typing_a_whole_page_one_key_at_a_time_is_fast", () => {
  // The real shape of the cost: a person types a file, and every `>`
  // among those keystrokes runs the rule over everything before it.
  const page = `fn P() -> Element {\n    return <div>\n${"        <Card><CardTitle>hi</CardTitle></Card>\n".repeat(400)}    </div>;\n}\n`;
  const spent = took(() => {
    for (let at = 0; at < page.length; at += 1) {
      if (page[at] === ">") {
        closingFor(page.slice(0, at + 1));
      }
    }
  });
  assert.ok(spent < 3000, `took ${spent}ms`);
});

// ------------------------------------------------------- the safety net

test("onlyIndentationChanged_accepts_a_reindent_of_anything", () => {
  // The formatter's own guard, held to the same inputs as the rule it
  // guards: if it ever rejects a real `reindent`, Format Document
  // silently stops working.
  for (const text of AWKWARD) {
    assert.ok(
      onlyIndentationChanged(text, reindent(text, NORMAL)),
      `rejected its own output on ${JSON.stringify(text.slice(0, 60))}`,
    );
  }
});

test("onlyIndentationChanged_refuses_anything_that_lost_text", () => {
  assert.equal(onlyIndentationChanged("a\nb", "a"), false);
  assert.equal(onlyIndentationChanged("a\nb", "a\nb\nc"), false);
  assert.equal(onlyIndentationChanged("a\nb", "a\nB"), false);
  assert.equal(onlyIndentationChanged("hello", "h ello"), false);
  // Joining two lines is a loss even though every character survives.
  assert.equal(onlyIndentationChanged("a\nb", "ab"), false);
});

test("onlyIndentationChanged_ignores_the_indentation_and_the_endings", () => {
  assert.equal(onlyIndentationChanged("a\n    b", "    a\nb"), true);
  assert.equal(onlyIndentationChanged("a\r\n    b", "a\n b"), true);
  assert.equal(onlyIndentationChanged("\ta", "        a"), true);
});

test("onlyIndentationChanged_survives_what_is_not_a_string", () => {
  for (const nonsense of [undefined, null, 0, {}, []]) {
    assert.doesNotThrow(() => onlyIndentationChanged(nonsense, "a"));
    assert.equal(onlyIndentationChanged(nonsense, "a"), false);
    assert.equal(onlyIndentationChanged("a", nonsense), false);
  }
});
