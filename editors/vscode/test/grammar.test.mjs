// What the editor must see in a `.rsx`.
//
// A grammar is the one part of an extension that can be tested without
// VS Code: `vscode-textmate` is the same tokeniser the editor runs, so
// what these tests read is what the colours will be.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const require = createRequire(import.meta.url);
const here = dirname(fileURLToPath(import.meta.url));
const extension = join(here, "..");

const oniguruma = require("vscode-oniguruma");
const textmate = require("vscode-textmate");

/** The tokeniser the editor itself uses. */
async function grammar() {
  const wasm = await readFile(
    join(extension, "node_modules/vscode-oniguruma/release/onig.wasm"),
  );
  await oniguruma.loadWASM(wasm);
  const registry = new textmate.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (sources) => new oniguruma.OnigScanner(sources),
      createOnigString: (text) => new oniguruma.OnigString(text),
    }),
    // Rust itself is VS Code's own grammar, which is not here: what is
    // under test is what this extension adds, and the grammar must stay
    // readable without it.
    loadGrammar: async (scope) => {
      if (scope !== "source.rsx") return null;
      const written = await readFile(
        join(extension, "syntaxes/rsx.tmLanguage.json"),
        "utf8",
      );
      return textmate.parseRawGrammar(written, "rsx.tmLanguage.json");
    },
  });
  const found = await registry.loadGrammar("source.rsx");
  assert.ok(found, "the grammar did not load");
  return found;
}

/** Every token of a source, as `{ text, scopes }`. */
async function tokenise(source) {
  const of = await grammar();
  const tokens = [];
  let state = textmate.INITIAL;
  for (const line of source.split("\n")) {
    const result = of.tokenizeLine(line, state);
    for (const token of result.tokens) {
      const text = line.slice(token.startIndex, token.endIndex);
      if (text.trim() === "") continue;
      tokens.push({ text, scopes: token.scopes });
    }
    state = result.ruleStack;
  }
  return tokens;
}

const PAGE = [
  'let mut points = 0;',
  '',
  'fn Page() -> Element {',
  '    // bonjour',
  '    return <section class="p-4">',
  '        <p>{points}</p>',
  '        <button onClick={move |_| points += 1}>plus</button>',
  '    </section>;',
  '}',
].join("\n");

test("a_tag_is_a_tag", async () => {
  const tokens = await tokenise(PAGE);
  const section = tokens.find((token) => token.text === "section");
  assert.ok(section, "the tag was not tokenised");
  assert.ok(
    section.scopes.some((scope) => scope.startsWith("entity.name.tag")),
    `got ${section.scopes.join(" ")}`,
  );
  const attribute = tokens.find((token) => token.text === "class");
  assert.ok(
    attribute?.scopes.some((scope) =>
      scope.startsWith("entity.other.attribute-name"),
    ),
    `got ${attribute?.scopes.join(" ")}`,
  );
  // And the text between the tags is text, not an operator soup.
  const word = tokens.find((token) => token.text === "plus");
  assert.ok(word, "the text of a tag was lost");
});

test("rust_inside_braces_is_rust", async () => {
  const tokens = await tokenise(PAGE);
  const closure = tokens.find((token) => token.text === "move");
  assert.ok(closure, "`move` inside a brace was not tokenised");
  assert.ok(
    closure.scopes.some(
      (scope) => scope.includes("keyword") || scope.includes("storage"),
    ),
    `got ${closure.scopes.join(" ")}`,
  );
  // The brace is the way back into Rust: a word inside one is a word of
  // the language, never the name of a tag or of an attribute.
  assert.ok(
    closure.scopes.some((scope) => scope.startsWith("meta.embedded")),
    `got ${closure.scopes.join(" ")}`,
  );
  assert.ok(
    !closure.scopes.some(
      (scope) =>
        scope.startsWith("entity.name.tag") ||
        scope.startsWith("entity.other.attribute-name"),
    ),
    `got ${closure.scopes.join(" ")}`,
  );
});

test("a_let_mut_is_a_keyword_everywhere", async () => {
  const tokens = await tokenise(PAGE);
  // At the top of the file, where it is the application's own state.
  const outside = tokens.filter((token) => token.text === "let");
  assert.ok(outside.length > 0, "`let` was not tokenised");
  for (const token of outside) {
    assert.ok(
      token.scopes.some((scope) => scope.includes("keyword")),
      `got ${token.scopes.join(" ")}`,
    );
  }
  const mutable = tokens.find((token) => token.text === "mut");
  assert.ok(
    mutable?.scopes.some((scope) => scope.includes("storage") || scope.includes("keyword")),
    `got ${mutable?.scopes.join(" ")}`,
  );
});

test("a_comment_is_a_comment_and_a_string_is_a_string", async () => {
  const tokens = await tokenise(PAGE);
  const comment = tokens.find((token) => token.text.includes("bonjour"));
  assert.ok(
    comment?.scopes.some((scope) => scope.startsWith("comment")),
    `got ${comment?.scopes.join(" ")}`,
  );
  const value = tokens.find((token) => token.text.includes("p-4"));
  assert.ok(
    value?.scopes.some((scope) => scope.startsWith("string")),
    `got ${value?.scopes.join(" ")}`,
  );
});

test("a_real_page_is_tokenised_whole", async () => {
  // The landing page of an app in this repository, which is the longest
  // `.rsx` there is: a grammar that leaves a string or a tag open runs
  // the rest of the file together into one colour.
  const page = await readFile(
    join(extension, "../../apps/hybrid/app/page.rsx"),
    "utf8",
  );
  const tokens = await tokenise(page);
  assert.ok(tokens.length > 100, `only ${tokens.length} tokens`);
  const last = tokens[tokens.length - 1];
  assert.ok(
    !last.scopes.some(
      (scope) => scope.startsWith("string") || scope.startsWith("comment"),
    ),
    `the file ends inside a ${last.scopes.join(" ")}`,
  );
  assert.ok(
    tokens.some((token) =>
      token.scopes.some((scope) => scope.startsWith("entity.name.tag")),
    ),
    "no tag was found in a page full of them",
  );
});
