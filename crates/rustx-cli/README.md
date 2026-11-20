# rustx-cli

**The `rustx` command.** This crate is the command line of
[rustx](https://github.com/AZTRA2022/rustx), a framework for native
desktop software written in Rust and drawn by a browser engine.

```sh
cargo install rustx-cli
```

The binary is called `rustx`. The package is `rustx-cli` because the
library beside the binary is reusable and the binary's name is the one a
developer types.

## The commands

```text
rustx new <name>      write a project that compiles the first time
rustx dev             watch the files, rebuild, bring the window back
rustx build           build, and give the .exe its own name and icon
rustx build --release the build that ships, and what ships beside it
rustx run             build and run once, without watching
rustx run --release   the same, from the release build
rustx --version
rustx --help
```

Nothing after a command is ignored. A word this does not take is named,
with a suggestion when it is one letter away from one that is — a
`rustx build --relese` that silently built in debug is the kind of
failure a command line has no excuse for.

Exit codes: `0` it worked, `1` the work failed, `2` what was typed was
not a thing to do.

## `rustx new`

```sh
rustx new my-app
cd my-app
rustx dev
```

Writes the project into the directory you are in, under the name you
gave it. There is no flag for anywhere else; `cd` first.

Eleven files, and every one of them has a reason:

```text
Cargo.toml          one dependency: rustx-app
src/main.rs         four lines: a window's title and its size
app/page.rsx        the first page, a counter
assets/
  tailwind.css      the whole of Tailwind, and shadcn/ui's theme
  readme.md
lib/utils.ts        the `cn` every shadcn component imports
components/
  readme.md         where `npx shadcn add` writes, and where you write
package.json        the libraries, and the install scripts approved
tsconfig.json       the `@/*` alias, which is how `@/lib/utils` resolves
components.json     what `npx shadcn add` reads before it writes
.gitignore
```

That is what makes `npm install recharts` and `npx shadcn add button
card` work on the day the project is born, with no `init` step and no
questions asked in a terminal that may not be interactive. A name cargo,
npm or Windows would refuse is refused here first and in words, before
a single file is written.

## `rustx dev`

Rebuilds on save and puts the new window where the old one was. The
order matters: the previous process is killed only once the new binary
exists, so a typo leaves what is on screen on screen, showing the last
page that compiled.

What survives a relaunch: the shared state declared with a `let mut` at
the top of a page, and the page that was open. What does not: the cells
inside a component, whose identity is a place in code that has just
changed. It relaunches the application; it does not reload native code
with the process still alive, and it says so rather than implying
otherwise.

## `rustx build`

It does not replace cargo, it calls it — and then finishes the job cargo
does not do: the `.exe` gets the application's name, version and icon as
Windows resources, read from where the developer already wrote them (the
version in `Cargo.toml`, the name and the logo in the `Config` of
`main.rs`). A second file to declare them in would be a second file to
forget to change. On Linux it writes a `.desktop` entry beside the
binary; on macOS a minimal `.app` bundle.

`cargo build` still works. The binary it makes simply looks anonymous in
an explorer window.

Stdout is the one path to the binary, because a script reads it. The
binary draws with the system's webview, so it is the application: the
assets it reads travel beside it in `rustx package`.

## What it needs on the machine

**Cargo, and nothing else.** No node, no npm, no Python, no shell. It
reads `$CARGO` first — the one thing in the environment that names a
cargo for certain, because cargo sets it for everything it starts — and
otherwise `cargo` on the `PATH`, and it says which it looked for when it
is not there.

Where it builds is cargo's business and cargo's alone, read in cargo's
own order: `CARGO_TARGET_DIR`, then `CARGO_BUILD_TARGET_DIR`, then
`[build] target-dir` in the nearest `.cargo/config.toml` at or above the
project, then `target/` at the root of the workspace. So it looks for the
binary where cargo wrote it, including from inside a workspace and
including with a shared build directory.

A project that reaches for a JavaScript library needs npm, and says so
itself when it does: the tools are run out of that project's own
`node_modules`, never off the `PATH`.

## Using it as a library

Every command is a function, so that what the command line does can be
tested without a command line:

```rust,ignore
use std::path::Path;

// Write a project, and get back every file that was written.
let written = rustx_cli::new::write("my-app", Path::new("."))?;

// Where cargo builds this project, by cargo's own rules.
let target = rustx_cli::build::target_directory(Path::new("my-app"));
```

`rustx_cli::new`, `build`, `dev`, `watch`, `resources`, `arguments` and
`help` are all public. The resource stamping in `resources` is
self-contained — the Windows version resource and icon group are built
by hand from three Win32 calls, with no `build.rs` and no dependency.

## The workspace

| Crate | What it is |
| --- | --- |
| `rustx-app` | what an application depends on |
| `rustx-dom` | the element tree, state, routes |
| `rustx-compiler` | `.rsx` → Rust, and the JavaScript bundle |
| `rustx-webview` | the host on the system's webview |
| `rustx-macro` | `#[rustx::main]` |
| `rustx-os` | the clipboard, dialogs, the tray, devices |
| **`rustx-cli`** | **this one** |
| `rustx-lsp` | the language server an editor starts |

This crate depends on `rustx-compiler` alone, for the names of the files
the build generates. It does not link the browser engine: a command line
that loaded CEF to read an application's name would be absurd.

Rust 1.88 or newer, edition 2024. Licensed under MIT or Apache-2.0, at
your option.
