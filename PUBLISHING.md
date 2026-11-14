# Publishing

Eight crates go to crates.io, and they depend on each other, so they go
in an order. Each one's manifest asks for its siblings by an exact
version — `rustx-dom = { path = "../rustx-dom", version = "0.3.0" }` —
so a crate cannot be published before the siblings it names are there.
`cargo package` says so plainly when you get it wrong:

    failed to select a version for the requirement `rustx-compiler = "^0.3.0"`
    candidate versions found which didn't match: 0.2.0, 0.1.1, 0.1.0

## The order

    1. rustx-dom         nothing internal
    2. rustx-compiler    nothing internal
    3. rustx-os          nothing internal
    4. rustx-macro       rustx-compiler
    5. rustx-engine      rustx-dom, rustx-os
    6. rustx-app         rustx-dom, rustx-engine, rustx-macro, rustx-os, rustx-compiler
    7. rustx-cli         rustx-compiler, rustx-dom
    8. rustx-lsp         rustx-compiler

The first three can go in any order among themselves, and in parallel.
After each publish, crates.io takes a moment to index; `cargo publish`
of the next one fails until it has, and the fix is to wait and run it
again, not to change anything.

`rustx` is taken on crates.io, so the package is **`rustx-app`** and the
library inside it is `rustx` — an application writes `rustx::` and never
sees the difference. `rustx-macros` is likewise the directory and
**`rustx-macro`** the package.

## Before any of it

    cargo test --workspace
    cargo package -p <each> --no-verify

`--no-verify` checks the manifest and the file list without building,
which is the half that catches the mistakes that are actually made:
a missing file, a path dependency with no version, a `readme` that is
not there.

## The version

One number for all eight, in `[workspace.package]` at the root. Bumping
it means bumping the pinned `version = "…"` on every internal
dependency in `crates/*/Cargo.toml` too — they are exact, deliberately,
because a mixed set of these crates is not a thing anybody wants to
debug.

`rustx new` asks for the minor of whatever command line wrote the
project (`wanted_version` in `crates/rustx-cli/src/new.rs`), read from
`CARGO_PKG_VERSION` rather than written out, so there is nothing to
update there and nothing to forget.

The applications under `apps/` are all `publish = false` and depend on
the framework by bare path, so none of this touches them.

## What is not published

`apps/`, `site/`, `editors/`. Only `crates/`.
