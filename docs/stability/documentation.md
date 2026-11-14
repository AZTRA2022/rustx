# Documentation — stability mission, §9 and gate 8

Owner: the documentation agent. Paths owned: `site/**`, `README.md`, `BUG.md`.
Everything else in the repository is read-only for this agent.

The standard this work is held to, from STABILITY.md §9:

> Every documented feature must correspond to actual working behavior. Every
> example must be reproducible. Do not document planned features as implemented
> features. The documentation should communicate technical confidence, not
> marketing promises.

## State

| | |
|---|---|
| Audit of `site/content/docs/**` | complete — 73 pages read against the code |
| Inaccuracies fixed | see the table below |
| Gaps filled | see the table below |
| `README.md` rewritten in English, French preserved as `README.fr.md` | pending |
| `BUG.md` normalised | pending |
| `npx next build` / `npx tsc --noEmit` | to re-run after every change |
| Example compilation | fixture project built; see "Compiling the examples" |

## How verification is done here

- **Site build.** `npx next build` and `npx tsc --noEmit` in `site/`. No network:
  `node_modules` is the one already in the tree.
- **Cargo.** Every cargo run goes through PowerShell, never Git Bash — Git Bash's
  `/usr/bin/link` shadows MSVC `link.exe`. Two variables in every such call:
  - `$env:CARGO_TARGET_DIR = "C:\Users\aztra\AppData\Local\Temp\rustx-target-docs"`,
    so the five other agents in this checkout are not blocked on one target lock;
  - `$env:CEF_PATH = "C:\Users\aztra\projects\rustx\target\debug\build\cef-dll-sys-dc8b90772f76ef75\out\cef_windows_x86_64"`,
    the CEF 156.0.14 archive already on disk. Without it a fresh target dir makes
    `cef-dll-sys`'s build script download CEF, which fails offline.
  - `$env:RUSTX_SOFTWARE = "1"` to run anything with a window: no GPU here.
- **Fixture project** for compiling documentation snippets, outside `apps/`:
  `C:\Users\aztra\AppData\Local\Temp\claude\d801b6bc7f540902\4758909d-b4ac-4211-8206-424f3b4785ed\scratchpad\fixture`
  — an ordinary `rustx new`-shaped project with its own `[workspace]` table so it
  does not join the repository's workspace, and a path dependency on
  `crates/rustx`. Snippets go into `app/*.rsx` and `src/`, and `cargo check`
  compiles them through `#[rustx::main]`, which is the real path a page takes.

## The audit, page by page

Legend: **ok** = every API named exists with that signature and every claim held
against the code at the time of reading; **fixed** = an inaccuracy was found and
corrected; **rewritten** = the page was materially wrong and replaced.

(filled in as the work proceeds — see "Findings" below for the detail)

## Findings — inaccuracies found

| where | what was wrong | evidence |
|---|---|---|
| `getting-started/project-structure.mdx` ×2, `tooling/configuration.mdx`, `tooling/upgrading.mdx` ×2, `async/network-calls.mdx` | `version = "0.2"` / "the workspace is at `0.2.0`" | `Cargo.toml` `[workspace.package] version = "0.3.0"` |
| `getting-started/installation.mdx`, `getting-started/project-structure.mdx` | "Five files, and a window" — the tree lists five entries | `crates/rustx-cli/src/new.rs:30-39` writes ten: `Cargo.toml`, `src/main.rs`, `app/page.rsx`, `.gitignore`, `assets/readme.md`, `package.json`, `tsconfig.json`, `components.json`, `lib/utils.ts`, `assets/tailwind.css` |
| `getting-started/assets.mdx` | "`rustx new` writes `assets/` with a short note and nothing else" | it also writes `assets/tailwind.css` (`new.rs:39`) |
| `the-router/parameters.mdx`, `the-router/special-files.mdx` | "A parameter arrives **decoded**: `/search/caf%C3%A9` gives `café`" | false. `captures()` in `crates/rustx-dom/src/route.rs:356` pushes the raw segment; `decoded()` (line 230) is only called from `parsed_query`, and its own doc says "which is the only place this is used". Only **query** values are decoded. Reported to the compiler agent as a code bug |
| `tooling/the-editor.mdx` | `code --install-extension editors/vscode/rustx-0.3.0.vsix` | the file is `editors/vscode/rustx-framework-extension-0.3.0.vsix` |
| `the-system/platforms.mdx` | "one folder per system inside — `windows/`, `linux/`, `other/`" and `other/clipboard.rs` = "macOS and the rest"; state table says Linux and macOS merely "compile; each call refuses" | `crates/rustx-os/src/` has a full `macos/` directory (24 files); `other/` holds only `mod.rs`; and per `lib.rs` the clipboard, notifications, autostart, a single instance, file dialogs and the camera list are *real* on both, plus window control on Linux via `x11rb` |
| `the-system/index.mdx`, `the-system/platforms.mdx`, `reference/index.mdx`, `tooling/contributing.mdx` | "Each module answers `supported()`" / "Every one has `supported() -> bool`" | only ten of the twenty-four modules do: `about`, `autostart`, `clipboard`, `dialog`, `instance`, `notify`, `shortcut`, `tasks`, `tray`, `window`. The twelve newest and `terminal` do not |
| `the-system/security.mdx` | "There is no secure storage module, no keychain access" | `crates/rustx-os/src/secret.rs` exists, is backed by `CredWriteW` on Windows, and is exercised end to end by `crates/rustx-os/tests/secret.rs` |
| `tooling/environment.mdx` | "There is no secure storage module yet" | same |
| `the-system/devices.mdx`, `whats-left.mdx` | "Recording and capture … nothing is written" | `crates/rustx-os/src/capture.rs` (screenshot of the desktop, a rectangle or a window) and `audio.rs` exist, with Windows implementations |
| `getting-started/migrating.mdx` | "secure storage — not written", "the app's data folder — none", "camera, microphone, screens — `devices`, then `getUserMedia`" | `secret`, `system`, `screen`, `audio`, `capture` now exist |
| `the-window/index.mdx`, `tooling/configuration.mdx`, `reference/index.mdx` | the `Config` table has no `without_scrollbars()` | `crates/rustx/src/lib.rs:266` |
| `getting-started/the-rsx-file.mdx` | the `style={{ … }}` section repeats the object form with no word about the literal-string rule, while `attributes-and-events.mdx` has the full section | the compile-time reading is real: `crates/rustx-compiler` refuses a non-literal inside an object literal |

## Findings — gaps (documented nowhere)

| what | where it lives | state of the page |
|---|---|---|
| Twelve OS capabilities: `about`, `attention`, `audio`, `bluetooth`, `capture`, `color`, `net`, `print`, `screen`, `secret`, `shell`, `system` | `crates/rustx-os/src/*.rs`, declared in `lib.rs` | pending |
| `rustx::terminal` — `shell()`, `run(script, at)`, `open(dir)`, `Shell`, `Output { code, out, err }` | `crates/rustx-os/src/terminal.rs`, re-exported at `crates/rustx/src/lib.rs:62` | pending |
| `Config::without_scrollbars()` | `crates/rustx/src/lib.rs:266` | pending |
| `rustx::os::window::drag_out(pane, paths)` | `crates/rustx-os/src/window.rs:169` | noted; `whats-left.mdx` still says dragging a file out is not written |

## Facade note, for the OS pages

The `rustx` facade re-exports only nine of the system modules by a short name:

```rust
pub use rustx_os::{autostart, clipboard, devices, dialog, instance, notify, shortcut, terminal, tray};
```

`crates/rustx/src/lib.rs:62`. The twelve newest capabilities are reachable only
through `rustx::os::` — `rustx::os::audio::volume()`, `rustx::os::screen::screens()`
and so on. The pages must write them that way. Reported to the runtime agent as
an ergonomics gap, not fixed here.

## Code bugs found and left for their owner

| file:line | what | owner |
|---|---|---|
| `crates/rustx-dom/src/route.rs:356` | a path parameter is handed to the page percent-encoded; only query values are decoded. `/search/caf%C3%A9` gives `caf%C3%A9` | compiler agent (`rustx-dom`) |
| `crates/rustx-cli/src/new.rs:432` | the page `rustx new` generates says `Bonjour.` — French in the first screen an English-speaking user sees | CLI agent |
| `crates/rustx-cli/src/new.rs` (`tailwind_css`) | the generated `assets/tailwind.css` declares no `--font-sans` and no `font-family` rule, while `site/.../tailwind.mdx` presents the system-font stack as what the theme block carries | CLI agent |
| `crates/rustx/src/lib.rs:62` | the facade does not re-export `about`, `attention`, `audio`, `bluetooth`, `capture`, `color`, `net`, `print`, `screen`, `secret`, `shell`, `system` | runtime agent |

## `BUG.md` — the `font-family` item

Investigated. It does **not** reproduce as written. Two separate things were
conflated in the one line:

1. a page's own `font-family` declaration does reach the document —
   `apps/counter/tests/app.rs::the_bodys_own_stylesheet_reaches_the_document`
   asserts `getComputedStyle(document.body).fontFamily == "system-ui, sans-serif"`;
2. what used to show was Times New Roman, because `BrowserSettings` left every
   generic font family empty and Chromium's fallback for an empty one is Times
   New Roman on every platform. Fixed in commit `14f8a27`,
   `crates/rustx-engine/src/cef_app.rs::browser_settings()`, which names
   `Segoe UI` / `Georgia` / `Consolas` on Windows. The comment there says in so
   many words that this was "the whole bug report".

Remaining real defect, which is why the item is not simply closed: the
`assets/tailwind.css` that `rustx new` writes declares no `--font-sans` and no
`font-family`, so a new project shows Chromium's default sans and the Tailwind
page describes a theme block the generated file does not have. Left for the CLI
agent, recorded above.

## Compiling the examples

(filled in as snippets are compiled)

## What remains unverified

(filled in at the end)
