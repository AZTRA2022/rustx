# Changelog

## 0.5.2 (2026-11-22)

### Fixed

- **A click that writes a value from itself panicked.** `store.open = !store.open` (and `n += n`) read the page value inside the write borrow of `update`, so the first click ended in "already borrowed" and the window went. The right-hand side is now read before the write: from a snapshot taken just before (`let __r = store.get(); store.update(|__v| { __v.open = !__r.open; })`), or, for a compound assignment, evaluated into a binding first.

## 0.5.1 (2026-11-22)

Found by building a large application (a 144-level simulation, a dozen pages, a shared state of forty fields) on 0.5.0.

### Fixed

- **A handler of several parameters that writes a page value got only the first argument.** The state rewrite wraps such a closure in a block (`{ let store = store.clone(); move |a, b| … }`) and the emitter, reading from the front, counted one parameter and saw no type annotation. It now starts from the closure itself.
- **`<Input onInput={move |event| …}>` was `type annotations needed`.** A prop of a component whose name is a DOM event (`onInput`, `onClick`, `onKeyDown`, …) is read as an `Event`, as it already was on a lowercase tag. A type written on the first parameter (`|v: rustx::Value|`) still wins, and a prop that is not a DOM event (`onValueChange`) is still a `Value`. `rustx::Event` is exported at the root.
- **Reading one field copied the whole page value.** `store.level` was `store.get().level`; it is now `store.with(|__s| __s.level.clone())`, and `let x = store.name.as_str()` binds the cloned field rather than the whole value.
- **`let outcome = store.exec()` with `exec(&mut self)` was `E0596`, or silently ran on a copy.** The methods that take `&mut self` are found in the application's own source, and calling one on a page value goes through the cell (`update_with`), where the change lands. New: `State::update_with` and `Shared::update_with`, which return what the change computed.
- **Editing a script under `components/` or `lib/` did not rebuild the application.** Nothing named those files, so the old bundle stayed in the binary. They are named now.

### Known

- A mutating method is recognised by name, from the application's own source: a name written both with `&self` and `&mut self` is left as a read, and a name that mutates on one type and is a plain read on another is not told apart. Calling such a method inside a render (not a handler) marks the component for the next frame, every frame.

## 0.5.0 (2026-11-21)

The workspace, the editor extension and the language server are all at 0.5.0.

### Breaking

- **The engine is the system's web view** (WebView2, WKWebView, WebKitGTK, through `wry` and `tao`). The Chromium Embedded Framework is gone: there is no `libcef`, no engine folder beside the executable, and a counter is about 2 MB. What a page can decode and draw is what the system's web view can.
- **There is no separate setup program.** `rustx package` stamps the application binary so that it installs itself (Windows, macOS, Linux), updates itself and uninstalls itself with `--uninstall`. `RUSTX_INSTALLER=inno` still writes an Inno Setup script. A `main.rs` that wants to stay portable says `Config::portable()`.
- **Every window starts with Inter and a default theme** (CSS variables `--rx-*`, in a CSS layer so an application's own rules win). The default not-found and error screens are Next.js's. The text of the not-found screen is now "This page could not be found."
- `Config::version`, `description` and `publisher` default to what `Cargo.toml` says.

### Added

- **`system/` pages**, in locked file names: `install`, `update`, `uninstall`, `license`, `permissions`, `about`, `settings`. The framework ships a default for each, in `rustx::setup` terms. `RUSTX_START=/__system/about` opens one to look at it.
- **Updates from the settings screen** (`Config::updates("owner/name")`): check, download with the digest verified, install, restart. `RUSTX_UPDATE_API` points the check elsewhere.
- **A missing WebView2 runtime is offered once**, downloaded and installed, instead of ending in an error.
- **Window control**: `Config` takes size limits, position, `centered`, `always_on_top`, `remember_window`, `background`, `frameless`; `rustx::window` has setters and `is_frameless()`; the second window is ordered over a pipe (`set_title`, `set_size`, `set_position`).
- `Config::redirect(from, to)`, `Config::capabilities(&[..])` (the natives a page may call), `Config::permissions(&[..])` and the macOS usage descriptions that follow from it; `rustx::screens::{not_found, forbidden, unauthorized, error_screen}`.
- A `<title>` in a page names the window while the page is shown.
- **Packaging**: the licence is carried inside the binary; the install stamp survives Windows signing; macOS signing (`RUSTX_MAC_IDENTITY`, `-` for ad hoc) and notarisation (`RUSTX_NOTARY_PROFILE`, never run against Apple's service); a Linux desktop entry with its icon; an update over a running copy.
- `rustx dev` watches `system/`.
- The editor: the language server answers for `system/*.rsx` as it does for `app/`; two snippets (`rtitle`, `rsystem`).
- Sample applications: the counter is the Next.js home page with RustX; Nemo Design (a node graph, a glTF viewport, a timeline, real export) and the video editor were rebuilt after their showcase images.

### Fixed

- A write inside a handler nested in conditional JSX (`{if … { <button onClick=…> }}`) compiled to a read.
- Two compilations of one crate (the binary and its tests) deleted each other's staged pages.
- A signed Windows executable lost its install stamp.

### Checked, and not

Run on Windows, Ubuntu (under Xvfb) and macOS on every push: the web view suite, the packaged application installing, updating and uninstalling itself, the seven `system/` screens, the frameless controls, the update check against a local release page. **Not** run: macOS notarisation, Microsoft's real WebView2 bootstrapper outside a CI step that has not yet passed, and any of it on a person's own desktop. See [what's left](site/content/docs/whats-left.mdx).
