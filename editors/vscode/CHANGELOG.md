# Changelog

## 0.5.0

Released with rustx 0.5.0, whose language server this is built against.

- **`system/*.rsx`** (the install, update, uninstall, licence, permissions,
  about and settings screens) get the same diagnostics, completion and
  go-to-definition as a page of `app/`: the server maps each to the page the
  build compiles it as.
- Two snippets: `rtitle` (a `<title>` that names the window) and `rsystem`
  (the skeleton of a `system/` page).

## 0.4.0

- **Snippets**, with the prefixes a React developer already types:
  `rfc` writes a component, `rpage` a screen, `rlayout` a layout,
  `useState` a value, `useEffect` an effect, `useMemo`/`useRef` a
  `keep`, `rmap` a keyed list, `rif` a condition. Each one inserts
  rustx rather than React, because a snippet that wrote React into a
  `.rsx` would be worse than no snippet at all. There are also ones for
  the system: a file dialog, the clipboard, a notification, this
  application's own data folder, an update check and a context menu.
- **Nine more things the editor can ask**, all answered by
  `rust-analyzer` behind the server and reported at the page's own
  coordinates: go to type definition, go to implementation, find all
  references, highlight occurrences, signature help, inlay hints,
  document symbols and folding. `rename`, code actions and formatting
  are refused on purpose — the first two answer with edits whose
  columns belong to the generated file, and the third would reformat a
  `.rsx` as Rust.
- **`#[rustx::main]` no longer goes red while you type.** The attribute
  ran the JavaScript bundler as part of its expansion, and
  `rust-analyzer` re-expands a proc macro on every keystroke: the
  bundler timed out, took the expansion with it, and every item below
  the attribute was reported as an error. An expansion asked for by the
  proc-macro server now skips it.

## 0.2.0

Released with rustx 0.2.0. The extension itself is unchanged; the
version follows the framework so that a reader can tell which server
it was built against.

## 0.1.3

- **Format Document** on a `.rsx`. It reindents and nothing else: no
  line is cut, joined or reordered, so it cannot change the meaning of
  what was written. It knows the difference between `<Card>` and
  `Vec<u32>`, indents the body of `task(move |x| {` one step and not
  two, keeps a method chain on its own step, and leaves the inside of a
  block comment exactly as it was.
- **Typing `>` closes the tag.** This was announced in 0.1.2 and was
  never wired: the setting existed, the rule existed, and nothing
  called it. It works now, and a test of the manifest against the
  source makes that particular mistake impossible to repeat.
- **Tailwind IntelliSense and Emmet work in a `.rsx`** with nothing to
  configure. Neither knows `.rsx` — it is too young — so the extension
  declares `rsx` for them as a default, which anybody can override.
- **Restart the language server** from the command palette. A server
  out of step with the compiler beside it no longer needs the window
  reloaded.
- A marketplace tile with the logo on it, honest categories (the
  extension claimed `Themes` and `Debuggers` and contributed neither),
  keywords, a changelog, and a declaration about untrusted folders —
  opening one starts `rustx-lsp`, and that should be said.
- A built `.vsix` left in the folder is no longer packaged inside the
  next one.

## 0.1.2

- The props of an npm tag are offered inside it: `interface ButtonProps`
  from a package's `.d.ts`, the destructured parameter of what
  `shadcn add` writes, or the type a declaration names, followed to its
  own object.

## 0.1.1

- The tags an application installed are offered after a `<`, read from
  `node_modules` and from the components the project wrote itself.

## 0.1.0

- The `rsx` language: `.rsx`, its comments, its pairs, the indentation
  after an open tag, and a grammar where Rust is Rust and a tag is a
  tag.
- The compiler's refusals, underlined on the line they were written on,
  while typing.
