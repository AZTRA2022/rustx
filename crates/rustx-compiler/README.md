# rustx-compiler

**Turns a `.rsx` file into ordinary Rust, and keeps the positions.** An
internal crate of [rustx](https://github.com/AZTRA2022/rustx).

> **You probably want the `rustx` crate instead** — the package
> `rustx-app`. You never call this crate from an application:
> `#[rustx::main]` calls it while it expands, which is how a page
> becomes part of the binary.
>
> It is published, and documented here, because two other things do call
> it: the command line (`rustx-cli`) and the language server
> (`rustx-lsp`). If you are writing an editor plugin, a linter or a
> build tool for `.rsx`, this is the crate you want.

## What a `.rsx` file is

Rust, with JSX written bare inside it — no wrapping macro:

```text
let mut clicks = 0;

fn Page() -> Element {
    return <main>
        <h1>Hello.</h1>
        <button onClick={move |_| clicks += 1}>
            Clicked {clicks} times
        </button>
    </main>;
}
```

There is no macro around that JSX on purpose. A macro is a wall
`rust-analyzer` cannot see through, and seeing through it is the whole
point: the generated file keeps the line numbers of the `.rsx`, so a
`rustc` error lands on the line the developer wrote rather than on a
line a build script invented.

## Using it

```rust,ignore
// A page, as Rust.
let rust = rustx_compiler::transpile::transpile(source)?;

// Or with the positions kept, which is what a language server needs.
match rustx_compiler::transpile::transpile(source) {
    Ok(rust) => { /* … */ }
    Err(error) => {
        // `error.span` is in the coordinates of the `.rsx`.
        eprintln!("{}:{}: {}", error.span.line, error.span.column, error.message);
    }
}
```

## What is in it

| Module | What it does |
| --- | --- |
| `scan` | finds the JSX inside the Rust, and where it is (`Span`) |
| `parse` | reads one element: tags, attributes, children, expressions |
| `attributes` | the attribute names and the shapes their values take |
| `tags` | which tags are HTML and which are an application's own |
| `emit` | writes the Rust that builds a `rustx_dom::Element` |
| `promote` | turns a `let mut` at the top of a page into shared state |
| `rewrite` | the rest of the file, with the JSX replaced |
| `routes` | the route table, read from the shape of `app/` |
| `map` | the position map that keeps `rustc` errors on the right line |
| `build` | the whole of it, for one project |
| `bundle` | the JavaScript: the entry point, esbuild, Tailwind |
| `transpile` | one file in, Rust out — the short way in |
| `error` | `CompileError`, which carries a `Span` |

### `bundle` is worth a word

An application declares its JavaScript libraries the way any JavaScript
project does — `npm install recharts` — and this module writes the one
file that imports them and hands them to the runtime, bundles it with
the application's **own** esbuild out of its **own** `node_modules`, and
runs the application's own Tailwind over the pages. Nothing is fetched,
nothing is run off the `PATH`, and the version of every tool is whatever
that project installed.

## Dependencies

`proc-macro2` and `quote`, and nothing else. No `syn`: the parser reads
the tokens by hand, because what it has to parse is not Rust — it is
Rust with something else inside it — and keeping the positions through
somebody else's AST is harder than reading the tokens.

Rust 1.88 or newer, edition 2024. Licensed under MIT or Apache-2.0, at
your option.
