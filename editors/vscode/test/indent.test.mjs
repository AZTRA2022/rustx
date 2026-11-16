// What Format Document does to a `.rsx`, and what it refuses to do.
//
// It reindents and nothing else: no line is cut, joined, or reordered,
// so the formatter cannot change the meaning of what was written. Every
// test here is therefore a pair of texts with the same lines in the same
// order, differing only in the spaces at the front.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

import { onlyIndentationChanged, reindent } from "../src/indent.mjs";

/** Every `.rsx` of the repository this extension lives in. */
function everyPage(at, into = []) {
  for (const name of readdirSync(at)) {
    if (name === "target" || name === "node_modules") {
      continue;
    }
    const path = join(at, name);
    if (statSync(path).isDirectory()) {
      everyPage(path, into);
    } else if (name.endsWith(".rsx")) {
      into.push(path);
    }
  }
  return into;
}

/** Four spaces per level, the way Rust is written. */
const rust = { tabSize: 4, insertSpaces: true };

test("a_block_is_one_level_in", () => {
  assert.equal(
    reindent("fn Page() -> Element {\nlet n = 0;\n}", rust),
    "fn Page() -> Element {\n    let n = 0;\n}",
  );
});

test("a_tag_is_one_level_in_and_its_closing_tag_comes_back_out", () => {
  assert.equal(
    reindent("return <Card>\n<CardTitle>Title</CardTitle>\n</Card>;", rust),
    'return <Card>\n    <CardTitle>Title</CardTitle>\n</Card>;',
  );
});

test("a_tag_that_closes_itself_changes_nothing", () => {
  assert.equal(
    reindent("<div>\n<br />\n<AArrowDown size={18} />\n</div>", rust),
    "<div>\n    <br />\n    <AArrowDown size={18} />\n</div>",
  );
});

test("attributes_written_across_lines_are_one_level_in", () => {
  assert.equal(
    reindent('<section\nid="waiting"\nclassName="p-8">\nok\n</section>', rust),
    '<section\n    id="waiting"\n    className="p-8">\n    ok\n</section>',
  );
});

test("a_rust_generic_is_not_a_tag", () => {
  // The whole difficulty of a `.rsx`: `Vec<u32>` must not open a level,
  // or everything under a declaration drifts right for ever.
  assert.equal(
    reindent("fn Page() -> Element {\nlet rows: Vec<u32> = Vec::new();\nlet n = 0;\n}", rust),
    "fn Page() -> Element {\n    let rows: Vec<u32> = Vec::new();\n    let n = 0;\n}",
  );
});

test("a_comparison_and_a_closure_arrow_are_not_tags", () => {
  assert.equal(
    reindent("fn f() {\nif a < b { ok(); }\nlet g = move |_| n += 1;\n}", rust),
    "fn f() {\n    if a < b { ok(); }\n    let g = move |_| n += 1;\n}",
  );
});

test("an_angle_bracket_inside_a_string_is_not_a_tag", () => {
  assert.equal(
    reindent('fn f() {\nlet s = "<p>not a tag</p>";\nlet n = 0;\n}', rust),
    'fn f() {\n    let s = "<p>not a tag</p>";\n    let n = 0;\n}',
  );
});

test("a_line_that_closes_then_opens_stays_where_it_was", () => {
  assert.equal(
    reindent("fn f() {\nif a {\nok();\n} else {\nno();\n}\n}", rust),
    "fn f() {\n    if a {\n        ok();\n    } else {\n        no();\n    }\n}",
  );
});

test("a_blank_line_gets_no_spaces", () => {
  assert.equal(
    reindent("fn f() {\n    \nlet n = 0;\n}", rust),
    "fn f() {\n\n    let n = 0;\n}",
  );
});

test("a_comment_is_indented_like_the_code_it_sits_in", () => {
  assert.equal(
    reindent("fn f() {\n// why\nlet n = 0;\n}", rust),
    "fn f() {\n    // why\n    let n = 0;\n}",
  );
});

test("the_inside_of_a_block_comment_is_left_exactly_as_it_was", () => {
  // A block comment can hold a drawing, a table, a quoted snippet. A
  // formatter that reflows it loses something it cannot put back.
  const source = "fn f() {\n/*\n    one column\n        and another\n*/\nlet n = 0;\n}";
  assert.equal(
    reindent(source, rust),
    "fn f() {\n    /*\n    one column\n        and another\n*/\n    let n = 0;\n}",
  );
});

test("an_angle_bracket_in_a_comment_opens_nothing", () => {
  assert.equal(
    reindent("fn f() {\n// see <Card> below\nlet n = 0;\n}", rust),
    "fn f() {\n    // see <Card> below\n    let n = 0;\n}",
  );
});

test("tabs_are_used_when_the_editor_asks_for_them", () => {
  assert.equal(
    reindent("fn f() {\nlet n = 0;\n}", { tabSize: 4, insertSpaces: false }),
    "fn f() {\n\tlet n = 0;\n}",
  );
});

test("nothing_is_cut_joined_or_reordered", () => {
  // The one law that makes a reindenter safe: the lines are the same
  // lines, in the same order, stripped of their leading spaces.
  const source = [
    "let mut n = 0;",
    "",
    "fn Page() -> Element {",
    "    async move {",
    "        send(n).await;",
    "    };",
    "    return <main>",
    '        <Button onClick={move |_| n += 1}>Clicked {n} times</Button>',
    "    </main>;",
    "}",
  ].join("\n");
  const formatted = reindent(source, rust);
  assert.deepEqual(
    formatted.split("\n").map((line) => line.trim()),
    source.split("\n").map((line) => line.trim()),
  );
});

test("a_page_already_formatted_is_returned_character_for_character", () => {
  // The exact-output test, against a fixture of this extension's own.
  //
  // It carries every shape the indenter has to get right, laid out the
  // way a Rust and JSX author lays it out. It is a fixture rather than a
  // real page because the assertion is about characters: a page in
  // `apps/` belongs to whoever is writing that application, and a test
  // that asserts the exact bytes of somebody else's file goes red for a
  // reason that has nothing to do with the formatter.
  //
  // What the real pages are still used for is the test below, whose
  // assertion holds of any valid `.rsx` whoever wrote it. Both are
  // needed: the sweep found four real defects in this formatter, and the
  // fixture is what keeps them fixed.
  const fixture = join(import.meta.dirname, "fixtures/formatted.rsx");
  const source = readFileSync(fixture, "utf8").replace(/\r\n/g, "\n");
  const formatted = reindent(source, rust);
  if (formatted !== source) {
    const was = source.split("\n");
    const now = formatted.split("\n");
    const moved = was
      .map((line, at) => [at, line, now[at]])
      .filter(([, line, after]) => line !== after)
      .map(
        ([at, line, after]) =>
          `${fixture}:${at + 1}\n  was: ${JSON.stringify(line)}\n  now: ${JSON.stringify(after)}`,
      );
    assert.fail(`the formatter moved lines:\n${moved.join("\n")}`);
  }
});

test("no_real_page_of_the_repository_loses_a_character_to_the_formatter", () => {
  // The sweep over the real pages, asserting only what is true of any
  // valid `.rsx` whoever wrote it: the formatter changes the spaces at
  // the front of a line and nothing else, keeps the endings the document
  // had, and keeps the number of lines.
  //
  // This is the very predicate `extension.ts` guards Format Document
  // with, so a page that failed it is a page the extension refuses to
  // format — which is the failure worth knowing about, and the one that
  // does not depend on anybody's taste in indentation.
  const pages = everyPage(join(import.meta.dirname, "../../../apps"));
  assert.ok(pages.length >= 8, `only ${pages.length} pages were found`);

  const broken = [];
  for (const page of pages) {
    const source = readFileSync(page, "utf8");
    const formatted = reindent(source, rust);
    if (!onlyIndentationChanged(source, formatted)) {
      broken.push(`${page} (text changed)`);
      continue;
    }
    if (source.includes("\r\n") !== formatted.includes("\r\n")) {
      broken.push(`${page} (line endings)`);
    }
    if (source.split(/\r?\n/).length !== formatted.split(/\r?\n/).length) {
      broken.push(`${page} (line count)`);
    }
  }
  assert.deepEqual(broken, [], `the formatter lost text in:\n${broken.join("\n")}`);
});

test("the_real_pages_of_the_repository_are_indented_as_this_formatter_would", () => {
  // And the strongest thing the real pages can be asked, without
  // asserting their exact bytes: how many of their lines the formatter
  // would move. Written as a ceiling rather than as zero, because a page
  // is allowed to be indented in a way its author preferred and this
  // formatter does not reproduce — but if that number climbs, the
  // formatter and the way this repository is actually written have
  // drifted apart, and one of them is wrong.
  //
  // It was at 17 pages with a moved line when the four defects this
  // suite now covers were found through it; it is a sweep worth keeping
  // for exactly that reason.
  const pages = everyPage(join(import.meta.dirname, "../../../apps"));
  const moved = [];
  for (const page of pages) {
    const source = readFileSync(page, "utf8").replace(/\r\n/g, "\n");
    const formatted = reindent(source, rust);
    if (formatted !== source) {
      const was = source.split("\n");
      const now = formatted.split("\n");
      const at = was.findIndex((line, index) => line !== now[index]);
      moved.push(
        `${page}:${at + 1}\n  was: ${JSON.stringify(was[at])}\n  now: ${JSON.stringify(now[at])}`,
      );
    }
  }
  const share = moved.length / Math.max(1, pages.length);
  assert.ok(
    share <= 0.1,
    `the formatter would move a line in ${moved.length} of ${pages.length} pages:\n${moved.join("\n")}`,
  );
});

test("a_boolean_continuation_is_written_one_step_in", () => {
  // `&&` and `||` at the start of a line continue the line above, the
  // same way a method chain does — which is how rustfmt writes it and
  // how the pages of this repository are written by hand. They were read
  // as statements of their own, so a condition split across lines came
  // back flush against the line that opened it.
  assert.equal(
    reindent("fn f() {\nif a\n&& b\n|| c\n{\nok();\n}\n}", rust),
    "fn f() {\n    if a\n        && b\n        || c\n    {\n        ok();\n    }\n}",
  );
  // A single `|` is a match arm's pattern and not a continuation.
  assert.equal(
    reindent("fn f() {\nmatch n {\n0\n| 1 => a(),\n_ => b(),\n}\n}", rust),
    "fn f() {\n    match n {\n        0\n        | 1 => a(),\n        _ => b(),\n    }\n}",
  );
});

test("a_line_that_calls_and_then_opens_a_block_indents_its_body", () => {
  // `match total(&rows) {` opens a parenthesis, closes it, and then
  // opens a brace. Only the first bracket a line opens is a level — so
  // that `task(move |_| {` puts its body one step in and not two — but a
  // bracket that had been closed on the same line was still counted, and
  // the brace after it was denied its level. Every arm of that `match`,
  // and the body of every `if let Some(x) = f(y) {`, came back flush
  // with the line above it.
  assert.equal(
    reindent("fn f() {\nmatch g(0) {\n0 => a(),\n_ => b(),\n}\n}", rust),
    "fn f() {\n    match g(0) {\n        0 => a(),\n        _ => b(),\n    }\n}",
  );
  assert.equal(
    reindent("fn f() {\nif let Some(v) = g(0) {\nuse_it(v);\n}\n}", rust),
    "fn f() {\n    if let Some(v) = g(0) {\n        use_it(v);\n    }\n}",
  );
  // And the rule it must not break: one line, two brackets still open,
  // body one step in.
  assert.equal(
    reindent("fn f() {\ntask(move |_| {\nwork();\n});\n}", rust),
    "fn f() {\n    task(move |_| {\n        work();\n    });\n}",
  );
});

test("a_self_closing_tag_puts_its_slash_at_the_level_of_its_name", () => {
  // `<Card`, its attributes, and then `/>` on a line of its own. The
  // `/>` belongs where `<Card` is, which is what every page in this
  // repository and every JSX formatter does; it was left at the level of
  // the attributes, one step too deep.
  assert.equal(
    reindent('<div>\n<Card\ntitle="a"\n/>\n</div>', rust),
    '<div>\n    <Card\n        title="a"\n    />\n</div>',
  );
  // And a `>` alone still comes back to the tag's level, with the
  // children one step in from there.
  assert.equal(
    reindent('<div>\n<Card\ntitle="a"\n>\nok\n</Card>\n</div>', rust),
    '<div>\n    <Card\n        title="a"\n    >\n        ok\n    </Card>\n</div>',
  );
});

test("a_tag_written_against_the_one_before_it_is_still_a_tag", () => {
  // `<Card><CardContent>x</CardContent></Card>` on one line. A `<`
  // straight after a `>` was refused outright, to keep `>>` and `=>`
  // from opening anything — so `<CardContent>` opened nothing while
  // `</CardContent>` closed something, and the line came out one frame
  // short. The `</Card>` after it then closed the enclosing block
  // instead, and every line below it in the file was placed against the
  // wrong thing.
  assert.equal(
    reindent(
      "fn f() {\nif a {\nreturn <Card><CardContent>x</CardContent></Card>;\n}\nlet n = 0;\n}",
      rust,
    ),
    "fn f() {\n    if a {\n        return <Card><CardContent>x</CardContent></Card>;\n    }\n    let n = 0;\n}",
  );
  // And `=>` and `>>` still open nothing.
  assert.equal(
    reindent("fn f() {\nlet n = a >> 2;\nlet m = 0;\n}", rust),
    "fn f() {\n    let n = a >> 2;\n    let m = 0;\n}",
  );
});

test("a_greater_than_inside_an_attribute_does_not_end_the_tag", () => {
  // `<button disabled={n + 1 >= all.len()} onClick={…}>`: the end of the
  // tag was found with `indexOf(">")`, which lands on the `>` of `>=`
  // inside the attribute. Everything after it was then read as if it
  // were outside the tag, and the `}` that closed the attribute closed
  // the element — so the children of that button lost their level.
  assert.equal(
    reindent(
      "<div>\n<button disabled={n + 1 >= all.len()} onClick={move |_| n += 1}>\nNEXT\n</button>\n</div>",
      rust,
    ),
    "<div>\n    <button disabled={n + 1 >= all.len()} onClick={move |_| n += 1}>\n        NEXT\n    </button>\n</div>",
  );
});

test("a_closing_tag_never_closes_a_block_and_a_brace_never_closes_a_tag", () => {
  // What makes the model survive a file somebody is in the middle of
  // typing: a delimiter closes a frame of its own kind or nothing at
  // all. A `</div>` that popped a `{` used to shift the whole rest of
  // the file one level out, and there is no state a half-typed file can
  // be in where that is the right answer.
  assert.equal(
    reindent("fn f() {\nlet s = </div>;\nlet n = 0;\n}", rust),
    "fn f() {\n    let s = </div>;\n    let n = 0;\n}",
  );
  assert.equal(
    reindent("<div>\n}\nok\n</div>", rust),
    "<div>\n    }\n    ok\n</div>",
  );
});

test("a_chain_inside_a_brace_is_not_indented_twice", () => {
  // `{cells` opens a level and the chain under it sits at that level:
  // the brace has already provided the step, and adding the chain's own
  // on top of it put every `.iter()` in the repository one level too
  // deep. A chain after a line that opened nothing still gets its step,
  // and the lines after the first match the first rather than walking
  // further right.
  assert.equal(
    reindent("<div>\n{cells\n.iter()\n.map(f)\n.collect()}\n</div>", rust),
    "<div>\n    {cells\n        .iter()\n        .map(f)\n        .collect()}\n</div>",
  );
  assert.equal(
    reindent("fn f() {\nlet d = rows\n.iter()\n.map(f)\n.collect();\n}", rust),
    "fn f() {\n    let d = rows\n        .iter()\n        .map(f)\n        .collect();\n}",
  );
  // And a chain that opens a block of its own still puts the body one
  // step in from the chain.
  assert.equal(
    reindent("fn f() {\nlet d = rows\n.iter()\n.map(|row| {\nwork(row);\n})\n.collect();\n}", rust),
    "fn f() {\n    let d = rows\n        .iter()\n        .map(|row| {\n            work(row);\n        })\n        .collect();\n}",
  );
});

test("a_line_inside_a_string_literal_is_left_exactly_as_it_was", () => {
  // A string broken across lines holds whatever its author put in it.
  // Re-indenting the second line of one changes the value of the
  // program's data — and for a string without a `\` at the break, it
  // changes what the program prints. Left alone, the same way the
  // inside of a block comment is.
  const escaped = [
    "fn f() {",
    "    let style = format!(",
    '        "display: flex; \\',
    '         border: 1px solid {}",',
    "        colour",
    "    );",
    "}",
  ].join("\n");
  assert.equal(reindent(escaped, rust), escaped);
  const raw = ["fn f() {", '    let s = "one', "       two", '    three";', "}"].join("\n");
  assert.equal(reindent(raw, rust), raw);
});

test("a_multiline_format_string_keeps_its_own_spaces", () => {
  // A string broken across lines with a `\` holds whatever spaces its
  // author put in it, and those are part of the value: moving them
  // changes what the program prints.
  const source = [
    "fn f() {",
    '    let style = format!(',
    '        "display: flex; \\\\',
    '        border: 1px solid {}\\",',
    "        colour",
    "    );",
    "}",
  ].join("\n");
  assert.ok(onlyIndentationChanged(source, reindent(source, rust)));
});

test("formatting_twice_changes_nothing_the_second_time", () => {
  // A formatter that is not idempotent fights the editor: save, format,
  // save again, and the file keeps changing.
  for (const page of everyPage(join(import.meta.dirname, "../../../apps"))) {
    const source = readFileSync(page, "utf8").replace(/\r\n/g, "\n");
    const once = reindent(source, rust);
    assert.equal(reindent(once, rust), once, `not idempotent: ${page}`);
  }
  const fixture = join(import.meta.dirname, "fixtures/formatted.rsx");
  const once = reindent(readFileSync(fixture, "utf8").replace(/\r\n/g, "\n"), rust);
  assert.equal(reindent(once, rust), once, "not idempotent: the fixture");
});

test("a_file_with_windows_line_endings_keeps_them", () => {
  // The editor hands over the text with the document's own endings. A
  // formatter that split on "\n" and trimmed would eat every "\r" and
  // rewrite the whole file as a side effect of indenting one line.
  assert.equal(
    reindent("fn f() {\r\nlet n = 0;\r\n}\r\n", rust),
    "fn f() {\r\n    let n = 0;\r\n}\r\n",
  );
});

test("a_file_already_formatted_is_returned_unchanged", () => {
  const source = [
    "let mut n = 0;",
    "",
    "fn Page() -> Element {",
    "    async move {",
    "        send(n).await;",
    "    };",
    "",
    "    return <main>",
    "        <p>{n}</p>",
    "    </main>;",
    "}",
    "",
  ].join("\n");
  assert.equal(reindent(source, rust), source);
});
