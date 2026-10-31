// What the extension promises VS Code, and whether it builds.
//
// Everything else an extension does needs VS Code running. What a test
// can hold on to is the manifest — the language, the activation, the
// setting that names the server — and the bundle: an extension that
// does not build is an extension that does nothing at all.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile, rm, stat } from "node:fs/promises";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ran = promisify(execFile);
const extension = join(dirname(fileURLToPath(import.meta.url)), "..");

async function manifest() {
  return JSON.parse(await readFile(join(extension, "package.json"), "utf8"));
}

test("the_extension_declares_the_language_and_its_server", async () => {
  const written = await manifest();

  const language = written.contributes.languages.find(
    (one) => one.id === "rsx",
  );
  assert.ok(language, "the language is not declared");
  assert.deepEqual(language.extensions, [".rsx"]);
  assert.ok(language.configuration, "the language has no configuration");

  const grammar = written.contributes.grammars.find(
    (one) => one.language === "rsx",
  );
  assert.equal(grammar.scopeName, "source.rsx");

  // Opening a `.rsx` is what starts the server, and nothing else is:
  // starting it when VS Code starts would run a server per window for
  // people who never open one.
  assert.ok(
    written.activationEvents.includes("onLanguage:rsx"),
    `got ${JSON.stringify(written.activationEvents)}`,
  );
  assert.equal(written.main, "./dist/extension.js");

  // Someone who built the server themselves must be able to say where
  // it is, or the extension only works for whoever has it on the PATH.
  const settings =
    written.contributes.configuration.properties["rustx.server.path"];
  assert.ok(settings, "the server path cannot be configured");
  assert.equal(settings.type, "string");

  // `vscode-languageclient` is the only thing shipped.
  assert.deepEqual(Object.keys(written.dependencies ?? {}), [
    "vscode-languageclient",
  ]);
});

test("the_bundle_builds", async () => {
  const bundle = join(extension, "dist/extension.js");
  await rm(bundle, { force: true });
  await ran(process.execPath, [join(extension, "build.mjs")], {
    cwd: extension,
  });
  const built = await stat(bundle);
  assert.ok(built.size > 0, "the bundle is empty");
  const text = await readFile(bundle, "utf8");
  // The one thing a bundle must not do is swallow `vscode`, which the
  // editor itself provides at runtime.
  assert.ok(text.includes("vscode"), "the bundle does not reach the editor");
});
