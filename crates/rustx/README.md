# rustx-app

**The one crate a rustx application depends on.** The package is
`rustx-app`; the library inside it is `rustx`, so your code and every
line of the documentation write `rustx::` and never see the difference.
(`rustx` is taken on crates.io by another project.)

rustx is a framework for native desktop software written in Rust and
drawn by a browser engine. Components are `.rsx` files — Rust with JSX
written bare inside it, no wrapping macro — compiled into the binary at
build time. There is no server, no WebView bridge to a JavaScript
application, and no `build.rs` in your project.

```toml
[dependencies]
rustx = { package = "rustx-app", version = "0.3" }
```

## The whole of a `main.rs`

```rust,ignore
use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("Console").size(900, 640))?;
    app.run()
}
```

`#[rustx::main]` compiles every `.rsx` under `app/` into the binary,
hands the router the route table it generated, and starts the
window. All of it is ordinary Rust that
could have been written by hand, which is what lets an application that
outgrows the attribute reach for `rustx_engine` directly and keep
going.

And a page, `app/page.rsx`:

```text
let mut clicks = 0;

fn Page() -> Element {
    return <button onClick={move |_| clicks += 1}>
        Clicked {clicks} times
    </button>;
}
```

The `let mut` at the top of a page is shared state. There is no
annotation, no `use`, and no macro around the JSX.

## What this crate is

A facade. It re-exports what an application names and nothing else:

| From | What an application writes |
| --- | --- |
| `rustx-dom` | `Element`, `Value`, `store`, `shared`, the route table |
| `rustx-engine` | `App`, `Config`, `task::spawn`, `time`, the window |
| `rustx-macro` | `#[rustx::main]`, `#[derive(ToValue)]` |
| `rustx-os` | `clipboard`, `dialog`, `notify`, `tray`, `screen`, … |
| `tokio` | `rustx::tokio`, so a page writes `rustx::time::sleep` |

Depending on it is how a `Cargo.toml` stays one line long. Nothing is
hidden: each of those crates is published on its own and documented on
its own, and you may depend on any of them directly.

## Getting started

```sh
cargo install rustx-cli    # the `rustx` command
rustx new my-app
cd my-app
rustx dev
```

`rustx new` writes a project that compiles the first time: a `main.rs`
of four lines, a page, and the `package.json`, `tsconfig.json` and
`components.json` that let `npm install recharts` or
`npx shadcn add button` work on the day the project is born.

## The workspace

| Crate | What it is |
| --- | --- |
| **`rustx-app`** | **this one — what an application depends on** |
| `rustx-dom` | the element tree, state, routes |
| `rustx-compiler` | `.rsx` → Rust, and the JavaScript bundle |
| `rustx-webview` | the host on the system's webview: the frame loop, the router, React |
| `rustx-macro` | `#[rustx::main]` |
| `rustx-os` | the clipboard, dialogs, the tray, devices |
| `rustx-cli` | `rustx new`, `dev`, `build`, `run` |
| `rustx-lsp` | the language server an editor starts |

All eight share one version number. The internal dependencies are
pinned exactly, deliberately: a mixed set of these crates is not
something anybody wants to debug.

## Platforms

Windows is where every capability is implemented and tested. Linux and
macOS build, and a documented subset of `rustx-os` is real on each —
see that crate's readme for exactly which. Rust 1.88 or newer, edition
2024.

Licensed under MIT or Apache-2.0, at your option.
