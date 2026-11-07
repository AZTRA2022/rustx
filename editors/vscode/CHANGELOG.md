# Changelog

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
