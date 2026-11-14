# rustx-lsp

**What an editor asks about a `.rsx`: where the mistakes are, and what
can be written here.** Part of
[rustx](https://github.com/AZTRA2022/rustx).

> You do not depend on this crate from an application. You **install**
> it, and your editor starts it:
>
> ```sh
> cargo install rustx-lsp
> ```
>
> The binary is `rustx-lsp`. It speaks the Language Server Protocol over
> standard input and standard output, which is how every editor speaks
> to one.

## Why it exists

A `.rsx` file is Rust with JSX written bare inside it. To `rust-analyzer`
alone that is a syntax error on the first `<`, and the file goes dark —
no diagnostics, no completion, no go-to-definition, for the one kind of
file the framework is about.

So this server compiles the file the editor is holding, with
`rustx-compiler`, and reports where the compiler refused it **in the
coordinates of the `.rsx`** — which is where `CompileError` already
points, because the compiler keeps a position map for exactly this.

## Using it

### VS Code

The extension in the rustx repository (`editors/vscode`) finds the
binary and starts it. It is the supported path, and it carries the
TextMate grammar for `.rsx` as well, so highlighting works before the
server has said anything.

### Any other editor

Point your LSP client at the `rustx-lsp` binary for the language id
`rsx`, over stdio. There is nothing to configure: no port, no
initialisation options, no workspace file.

### As a library

```rust,ignore
// The whole server, over anything that reads and writes.
rustx_lsp::serve(&mut std::io::stdin().lock(), &mut std::io::stdout().lock());
```

`serve_with` takes the pieces apart if you want to embed it.

## What it does

| Module | What it is |
| --- | --- |
| `rpc` / `wire` / `json` | the protocol, and a JSON reader written here |
| `documents` | the editor's buffers, which are the truth |
| `diagnostics` | the compiler's refusals, at the right line and column |
| `inside` / `picked` | where the cursor is, in the shape of the document |
| `suggest` | what can be written here: tags, attributes, components |
| `libraries` | the components an installed npm library offers |
| `shadow` / `crate_for_pages` | the Rust handed to `rust-analyzer` |
| `downstream` | `rust-analyzer` behind this server, and its answers back |

### It does not crash on half-typed code

Every handler is reached from a keystroke, on text that is mid-edit and
therefore not valid anything. The compiler is called on that text — to
underline it, to write the shadow, to find the region around the cursor
— and a compiler that panicked on a half-written tag would take this
process with it. A server that died once underlines nothing for the rest
of the session, in every file, with no message saying why: the one
failure a developer cannot work around and cannot diagnose.

So nothing a message handler computes is allowed to escape. Every one of
them runs inside a catch, and a panic becomes "no answer to this
request" rather than the end of the session.

## What it does not do yet

Completion, hovers and definitions for ordinary Rust inside a `.rsx`
come from putting `rust-analyzer` behind this same server and
translating its answers back through the position map. The shadow crate
and the downstream plumbing are here; it is not finished, and it is not
claimed to be. Diagnostics and `.rsx`-aware suggestion are what works
today.

## Dependencies

`rustx-compiler`, and nothing else. No `tower-lsp`, no `serde`: the
protocol is a content-length header and a JSON object, and the JSON this
server exchanges is read by hand in `json.rs`. A language server that
pulled a dependency tree behind it would be a language server a
developer waits for.

Rust 1.88 or newer, edition 2024. Licensed under MIT or Apache-2.0, at
your option.
