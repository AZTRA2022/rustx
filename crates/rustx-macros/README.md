# rustx-macro

**`#[rustx::main]`: everything a rustx application should not have to
write.** An internal crate of
[rustx](https://github.com/AZTRA2022/rustx).

> **You want the `rustx` crate instead** — the package `rustx-app`,
> which re-exports this attribute as `rustx::main`. Depending on this
> crate directly gets you an attribute whose generated code names
> `rustx_dom` and `rustx_engine`, so you would have to declare those
> too. There is no reason to.
>
> The package is `rustx-macro` because `rustx_macros` was taken on
> crates.io. The library is `rustx_macros`, so nothing in the code
> changes.

## What it writes

```rust,ignore
use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("Console").size(900, 640))?;
    app.run()
}
```

Around that, the attribute adds what does not vary between
applications: starting the window,
finding `assets/index.html`, compiling every `.rsx` under `app/` into
the binary, and handing the router the route table it generated. A
developer who writes those is reading the framework's own plumbing in
their own file.

`#[rustx::main(pages)]` compiles the pages without routing, for an
application that mounts its own components.

`#[derive(ToValue)]` is here too: it is how a struct crosses into
JavaScript as a `rustx_dom::Value`.

## Why there is no `build.rs`

The pages are compiled **here**, while the attribute expands, because
that is the one moment that happens every time the crate is compiled and
never otherwise. Each `.rsx` is then named by an `include_bytes!`, which
is what makes cargo and `rustc` watch it: change a page, and the
application is rebuilt. No build script in the developer's project,
which is precisely what this framework refuses.

The generated Rust is written to files and included by path rather than
pasted in as tokens. Pasting would have been shorter, but then a `rustc`
error about a developer's own Rust would point at this attribute instead
of at the line they wrote. The file keeps the line numbers of the
`.rsx`, so the error lands where they are looking.

## Dependencies

`proc-macro2`, `quote`, and `rustx-compiler` — which is what actually
turns a `.rsx` into Rust. This crate is the attribute and the file
layout; the compiler is in its own crate because a proc-macro crate can
export nothing but macros, and a compiler wants tests of its own.

Rust 1.88 or newer, edition 2024. Licensed under MIT or Apache-2.0, at
your option.
