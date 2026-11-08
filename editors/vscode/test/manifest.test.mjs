// What the manifest promises, against what the source does.
//
// A declared setting with no code behind it looks exactly like a
// working feature: the setting appears in the editor's UI, the tests of
// the rule pass on their own, and nothing anywhere says the wire was
// never connected. That happened here — `rustx.closeTags` was declared
// and documented, its fifteen tests were green, and `closingFor` was
// called from nowhere. These laws are what makes that impossible to
// repeat.

import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const here = import.meta.dirname;
const root = join(here, "..");
const manifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));

/** Everything the extension's own source says, as one text. */
function sources() {
  const found = [];
  const look = (at) => {
    for (const name of readdirSync(at)) {
      const path = join(at, name);
      if (statSync(path).isDirectory()) {
        look(path);
      } else if (/\.(ts|mjs|js)$/.test(name)) {
        found.push(readFileSync(path, "utf8"));
      }
    }
  };
  look(join(root, "src"));
  return found.join("\n");
}

test("every_setting_the_manifest_declares_is_read_somewhere", () => {
  const properties = manifest.contributes?.configuration?.properties ?? {};
  const code = sources();
  const unread = Object.keys(properties).filter((whole) => {
    // `rustx.closeTags` is read as `getConfiguration("rustx").get("closeTags")`,
    // so what has to appear in the source is the part after the section.
    const name = whole.replace(/^rustx\./, "");
    return !code.includes(`"${name}"`);
  });
  assert.deepEqual(unread, [], `declared and never read: ${unread.join(", ")}`);
});

test("every_command_the_manifest_declares_is_registered", () => {
  const commands = manifest.contributes?.commands ?? [];
  const code = sources();
  const missing = commands
    .map((one) => one.command)
    .filter((id) => !code.includes(`"${id}"`));
  assert.deepEqual(missing, [], `declared and never registered: ${missing.join(", ")}`);
});

test("the_marketplace_tile_has_an_icon_that_exists", () => {
  // `contributes.languages[].icon` is the icon of a *file*. The tile in
  // the marketplace and in the extensions list is the top-level one,
  // and an extension without it shows a grey square.
  assert.ok(manifest.icon, "the manifest declares no top-level icon");
  assert.ok(
    existsSync(join(root, manifest.icon)),
    `the icon ${manifest.icon} is not there`,
  );
});

test("the_categories_are_the_ones_actually_contributed", () => {
  // A category is a promise to somebody browsing. `Debuggers` with no
  // debugger and `Themes` with no theme are how an extension gets
  // installed and uninstalled in the same minute.
  const contributes = manifest.contributes ?? {};
  const earned = {
    "Programming Languages": () => (contributes.languages ?? []).length > 0,
    Linters: () => true,
    Formatters: () => true,
    Snippets: () => (contributes.snippets ?? []).length > 0,
    Themes: () =>
      (contributes.themes ?? []).length > 0 ||
      (contributes.iconThemes ?? []).length > 0,
    Debuggers: () => (contributes.debuggers ?? []).length > 0,
    "Language Packs": () => (contributes.localizations ?? []).length > 0,
  };
  const unearned = (manifest.categories ?? []).filter(
    (name) => earned[name] !== undefined && !earned[name](),
  );
  assert.deepEqual(unearned, [], `claimed and not contributed: ${unearned.join(", ")}`);
});

test("the_package_carries_the_icon_and_not_the_packages", () => {
  const ignored = readFileSync(join(root, ".vscodeignore"), "utf8");
  const lines = ignored.split("\n").map((line) => line.trim());
  assert.ok(
    lines.includes("*.vsix"),
    "a built .vsix in the folder would be packaged inside the next one",
  );
  assert.ok(
    !lines.some((line) => line === "icons/" || line === "icons"),
    "the icon has to ship, or the tile is a grey square",
  );
});

test("the_marketplace_has_something_to_show_beside_the_name", () => {
  assert.ok(manifest.description, "no description");
  assert.ok(manifest.repository?.url, "no repository");
  assert.ok((manifest.keywords ?? []).length >= 3, "fewer than three keywords");
  assert.ok(existsSync(join(root, "README.md")), "no README");
  assert.ok(existsSync(join(root, "CHANGELOG.md")), "no CHANGELOG");
  assert.ok(existsSync(join(root, "LICENSE")), "no LICENSE");
});

test("an_extension_that_launches_a_binary_says_so", () => {
  // rust-analyzer declares this for exactly this reason: opening a
  // folder must not run a program from it without the person agreeing.
  assert.ok(
    manifest.capabilities?.untrustedWorkspaces,
    "the extension starts `rustx-lsp` and declares nothing about untrusted folders",
  );
});

test("the_defaults_that_make_other_extensions_work_on_rsx_are_declared", () => {
  // `.rsx` is young, so Tailwind IntelliSense and Emmet do not know it.
  // Neither needs to: an extension can set defaults for another's
  // settings, so rustx names `rsx` for them and nobody configures
  // anything.
  const defaults = manifest.contributes?.configurationDefaults ?? {};
  assert.equal(defaults["tailwindCSS.includeLanguages"]?.rsx, "html");
  assert.ok(
    (defaults["tailwindCSS.classAttributes"] ?? []).includes("className"),
    "Tailwind would not look inside className",
  );
  assert.equal(defaults["emmet.includeLanguages"]?.rsx, "html");
});
