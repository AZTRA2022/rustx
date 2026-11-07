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

import { reindent } from "../src/indent.mjs";

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
    reindent("return <Card>\n<CardTitle>Titre</CardTitle>\n</Card>;", rust),
    'return <Card>\n    <CardTitle>Titre</CardTitle>\n</Card>;',
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
    reindent('<section\nid="attente"\nclassName="p-8">\nok\n</section>', rust),
    '<section\n    id="attente"\n    className="p-8">\n    ok\n</section>',
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
    reindent('fn f() {\nlet s = "<p>pas une balise</p>";\nlet n = 0;\n}', rust),
    'fn f() {\n    let s = "<p>pas une balise</p>";\n    let n = 0;\n}',
  );
});

test("a_line_that_closes_then_opens_stays_where_it_was", () => {
  assert.equal(
    reindent("fn f() {\nif a {\nok();\n} else {\nnon();\n}\n}", rust),
    "fn f() {\n    if a {\n        ok();\n    } else {\n        non();\n    }\n}",
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
    reindent("fn f() {\n// pourquoi\nlet n = 0;\n}", rust),
    "fn f() {\n    // pourquoi\n    let n = 0;\n}",
  );
});

test("the_inside_of_a_block_comment_is_left_exactly_as_it_was", () => {
  // A block comment can hold a drawing, a table, a quoted snippet. A
  // formatter that reflows it loses something it cannot put back.
  const source = "fn f() {\n/*\n    une colonne\n        et une autre\n*/\nlet n = 0;\n}";
  assert.equal(
    reindent(source, rust),
    "fn f() {\n    /*\n    une colonne\n        et une autre\n*/\n    let n = 0;\n}",
  );
});

test("an_angle_bracket_in_a_comment_opens_nothing", () => {
  assert.equal(
    reindent("fn f() {\n// voir <Card> plus bas\nlet n = 0;\n}", rust),
    "fn f() {\n    // voir <Card> plus bas\n    let n = 0;\n}",
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
    "        envoyer(n).await;",
    "    };",
    "    return <main>",
    '        <Button onClick={move |_| n += 1}>Cliqué {n} fois</Button>',
    "    </main>;",
    "}",
  ].join("\n");
  const formatted = reindent(source, rust);
  assert.deepEqual(
    formatted.split("\n").map((line) => line.trim()),
    source.split("\n").map((line) => line.trim()),
  );
});

test("the_real_pages_of_the_repository_are_left_as_they_are", () => {
  // The pages of `apps/` are written by hand and indented the way their
  // author wanted. A formatter that moved a single one of their lines
  // would be telling them they were wrong, and would be wrong itself.
  const pages = everyPage(join(import.meta.dirname, "../../../apps"));
  assert.ok(pages.length >= 8, `only ${pages.length} pages were found`);

  const moved = [];
  for (const page of pages) {
    const source = readFileSync(page, "utf8").replace(/\r\n/g, "\n");
    const formatted = reindent(source, rust);
    if (formatted !== source) {
      const was = source.split("\n");
      const now = formatted.split("\n");
      const at = was.findIndex((line, index) => line !== now[index]);
      moved.push(`${page}:${at + 1}\n  was: ${JSON.stringify(was[at])}\n  now: ${JSON.stringify(now[at])}`);
    }
  }
  assert.deepEqual(moved, [], `the formatter moved lines:\n${moved.join("\n")}`);
});

test("formatting_twice_changes_nothing_the_second_time", () => {
  // A formatter that is not idempotent fights the editor: save, format,
  // save again, and the file keeps changing.
  for (const page of everyPage(join(import.meta.dirname, "../../../apps"))) {
    const source = readFileSync(page, "utf8").replace(/\r\n/g, "\n");
    const once = reindent(source, rust);
    assert.equal(reindent(once, rust), once, `not idempotent: ${page}`);
  }
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
    "        envoyer(n).await;",
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
