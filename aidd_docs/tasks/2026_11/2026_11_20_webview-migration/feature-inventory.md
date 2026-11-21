# Tauri and Next.js features: what rustx has (2026-11-21)

Present = written and used; Partial = exists with a stated limit; Missing = not
there. Evidence is the module or doc page. "Run on" says where it was actually
exercised: Windows by hand and by tests; macOS and Linux on GitHub runners (rustx-os tests, packaging, and on Linux the whole webview suite under Xvfb); nothing driven on a real Mac or Linux desktop.

## Tauri, most essential first

| # | Feature | State | Evidence |
|---|---|---|---|
| 1 | System webview, small binary | Present | `rustx-webview` (wry/tao); counter folder 1.76 MB, setup 1.9 MB |
| 2 | Rust ⇄ page commands | Present | `register_native` (sync), `register_async_native` (future off the loop thread); `rustx-webview/tests/page.rs` |
| 3 | Events page → Rust with payloads | Present | `Event` (x, y, deltas, numbers…), coalesced bursts |
| 4 | Window control, frameless, multi-window | Partial | `rustx::window`: size, position, limits, focus, visibility, title, events, close guard, state, `with(|tao window|)`; a second window is a child process, so the first cannot control it |
| 5 | Filesystem access | Present | `rustx::data`, `files::allow/url` (file:// served by a route with grants and ranges) |
| 6 | Dialogs, clipboard, notifications, tray, menus, global shortcuts | Present | `rustx-os` modules; Windows run, others compile-checked |
| 7 | Single instance, autostart, deep-link scheme | Present | `os::instance`, `os::autostart`, scheme in desktop entry |
| 8 | Shell / terminal / sidecar processes | Partial | `os::terminal`, `os::shell`; no sidecar-binary bundling |
| 9 | Updater | Present | `rustx::update`, release manifest + digest |
| 10 | Bundler: installer per platform | Partial | The stamped binary installs itself on all three systems (system/*.rsx screens, no separate setup), Inno optional; Linux `.deb`; macOS `.pkg`. Only Windows run. No `.msi`, `.rpm`, AppImage, notarisation |
| 11 | Code signing | Partial | `RUSTX_SIGN_PFX/SUBJECT` through `signtool`; nothing else |
| 12 | Security model (allow-list, CSP-like) | Partial | `Policy` allow-list of origins, file grants by write, cross-site refusal; no per-command capability scopes |
| 13 | Secret storage | Present | `os::secret` (credential store) |
| 14 | Printing | Partial | `window::print` through the system dialog; no print-to-PDF (webviews cannot) |
| 15 | Devtools in debug only | Present | `Config.devtools`; context menu kept for text |
| 16 | Cross-compile | Partial | CI matrix builds on three systems; no `--target` cross build from one host |
| 17 | Window state persistence (size/position) | Present | `Config::remember_window`, tested by a two-run probe |
| 18 | Mobile (iOS, Android) | Missing | out of scope for the webview migration |
| 19 | Plugins ecosystem (store, sql, log, websocket) | Missing | `rustx::data` covers the store; the rest is a crate away |
| 20 | Window effects (mica, vibrancy), positioner | Missing | |

## Next.js, most essential first

| # | Feature | State | Evidence |
|---|---|---|---|
| 1 | File-system routing, layouts, nested routes | Present | `app/page.rsx`, `layout.rsx`, `[name]` segments |
| 2 | Client-side navigation, `router::navigate`, route state | Present | `router` |
| 3 | Error / not-found screens | Present | `router.rs` tests (error screen wins) |
| 4 | Component state, effects, tasks | Present | `let mut`, `effect`, `task(async …)`; a handler cannot start a task (ledger row 22) |
| 5 | React ecosystem (shadcn, Radix, Recharts…) | Present | `foreign.rs` boundary; bundle holds only the libraries a page names |
| 6 | Tailwind | Present | `bundle.rs` runs Tailwind over `app/**/*.rsx` |
| 7 | Fast refresh | Partial | asset edits reload the page in place with Rust state kept; a `.rsx` edit rebuilds and relaunches with named state carried; no in-place hot patch of native code |
| 8 | Static assets, fonts, images | Present | `assets/` (carried in the binary, live folder in `rustx dev`) |
| 9 | Server components / SSR / API routes | Missing | not applicable: there is no server; natives play the role of API routes |
| 10 | Image optimisation, `next/font`, `next/link` prefetch | Missing | |
| 11 | Metadata / head management per route | Partial | window title from `Config`; no per-route `<head>` |
| 12 | Middleware, redirects, rewrites | Missing | |
| 13 | Streaming / suspense | Missing | async natives settle promises; no suspense boundary |

## What this changes in the plan

Closest to the user's two goals ("Tauri-like", "Next.js-like fast refresh"):
rows Tauri 17 (window state), Tauri 4 (multi-window as threads, not processes),
Next 7 (native hot patch) and ledger row 22 (a task started from a handler) are
the gaps worth the next work. Everything marked Missing in the Tauri half other
than 17 is by design.
