# rustx-engine

**The CEF host a rustx window is drawn by: the frame loop, the router,
the React boundary.** An internal crate of
[rustx](https://github.com/AZTRA2022/rustx).

> **You probably want the `rustx` crate instead** — the package
> `rustx-app`, which re-exports `App`, `Config`, `task` and the window
> under `rustx::`, and brings the attribute that writes the ceremony
> around them.
>
> Depend on this one directly if your application has outgrown
> `#[rustx::main]` and wants to drive the host itself. Everything the
> attribute generates is ordinary code that could have been written by
> hand, and this is the crate it would have been written against.

## What it does

It runs Chromium (through [CEF](https://bitbucket.org/chromiumembedded/cef))
and makes the application's **own Rust code live inside the render
process**. A component is Rust; it returns a `rustx_dom::Element`; the
engine diffs that tree and touches the real DOM. There is no server, no
IPC round trip per event, and no JavaScript application on the other
side of a bridge.

| Module | What it is |
| --- | --- |
| `app` | `App`, `Config`: the window, the document, the cache directory |
| `bootstrap` | leaving the sub-process before it touches anything |
| `frame` | the frame loop, and when a tree is redrawn |
| `render` / `renderer` | the diff applied to the DOM |
| `router` | routes, navigation, history, the not-found page |
| `foreign` | the React boundary: mounting a JavaScript component |
| `bridge` / `value` | what crosses between Rust and the page |
| `task` / `runtime` | the tokio reactor a `Future` in a component awaits |
| `window` | the frame, the size, the icon, through `rustx-os` |
| `panics` / `guard` / `watchdog` | what happens when something goes wrong |

## The sandbox, said out loud

The render process hosts the application's own Rust code, so it is **not
sandboxed** — `no_sandbox: 1`, and the `cef` dependency is built with
`default-features = false` for the same reason. That is only defensible
because the engine refuses remote content: an application draws its own
pages, out of its own binary, from `file://`. `App::new` and `guard`
enforce it.

This is a real security trade-off and it is written down rather than
implied. If you need to display arbitrary remote content, this is not
the right engine for that window.

## Building it

The `cef` crate downloads and links a Chromium Embedded Framework
distribution, which is large — about 300 MB of `libcef.dll` in a debug
build — and the first build of a project is long for that reason. The
binary it produces is a Chromium host: `libcef.dll` and Chromium's
resources (`*.pak`, `icudtl.dat`, `v8_context_snapshot.bin`,
`locales/`) have to be beside the executable at run time. The directory
is the artefact, not the `.exe` alone.

`RUSTX_SOFTWARE=1` forces software rendering, for a machine with no GPU.

## Dependencies

`cef`, `tokio` (`rt-multi-thread`, `time`, `net`, `sync`, `macros`),
`rustx-dom` for the tree, and `rustx-os` for the one thing Chromium does
not do — commanding the window the operating system made.

Rust 1.88 or newer, edition 2024. Licensed under MIT or Apache-2.0, at
your option. CEF and Chromium carry their own licences.
