---
objective: "A rustx application renders in the system webview (wry + tao) instead of CEF, ships in a few MB, opens no terminal, and behaves as before for apps/counter and apps/hybrid."
status: implemented
---

# Plan: Migrate rustx from CEF to a system webview

Source: BUG.md (heavy ~200 MB hello world, terminal at launch, slow app, Tauri-like rendering expected) and the user's decision "on passe sur du Tauri-like".

## Phases

| #   | Phase        | File                         | Depends on |
| --- | ------------ | ---------------------------- | ---------- |
| 1   | Safety net and wire protocol | [`phase-1.md`](./phase-1.md) | none |
| 2   | tao + wry host | [`phase-2.md`](./phase-2.md) | 1 |
| 3   | Patch applier and event round trip | [`phase-3.md`](./phase-3.md) | 2 |
| 4   | Natives over IPC and task wake-up | [`phase-4.md`](./phase-4.md) | 3 |
| 5   | React boundary in JavaScript | [`phase-5.md`](./phase-5.md) | 3 |
| 6   | Window controls on tao | [`phase-6.md`](./phase-6.md) | 2 |
| 7   | Remove CEF | [`phase-7.md`](./phase-7.md) | 4, 5, 6 |

## External sources

| URL   | What it settled   |
| ----- | ----------------- |
| none | Nothing consulted online; wry and tao APIs must be read in the pinned version at the start of phase 2. |

## Impact

| Category                                           | Verdict and evidence | Handled in         |
| -------------------------------------------------- | -------------------- | ------------------ |
| runtime | Rust code ran inside CEF's render process, a native call was a same-thread call (renderer/mod.rs header). It becomes async IPC. register_native and on_frame are called by apps/video-editor, by the rustx facade (crates/rustx/src/lib.rs, four sites) and by crates/rustx-engine/src/render.rs, so the break reaches the facade and is wider than first counted. | 4 |
| runtime | Node reads in crates/rustx-engine/src/dom.rs are synchronous and public (text, attribute, tag_name, query_selector, count_matching). A webview cannot answer them synchronously. Unverified: who outside the engine calls them. | 3 |
| runtime | preventDefault and stopPropagation appear only in dropped.rs and its test, plus one asset in video-editor. The router's link interception is the other synchronous decision, to be moved to JavaScript. Unverified: router.rs not yet read in full. | 3 |
| regression | The onClick fix (Event::from_value on annotated component closures) stays valid because events keep crossing as Value records. | 5 |
| architecture | rustx-os takes a raw window handle through Pane::from_handle, so tao's handle can be given to it. | 6 |
| architecture | rustx-dom already has diff() and a Patch enum, reused as the wire format. | 1 |
| regression | rustx-engine tests depend on CEF through hello.rs (1394 lines). Unverified: which tests; to be listed in phase 1. | 1, 7 |
| runtime | BUG.md says the built application is extremely slow. Cause unmeasured: debug mode versus CEF start-up. Phase 1 records start-up and click-to-paint latency, phase 7 compares. Unverified until then. | 1, 7 |
| runtime | The launch terminal: the new.rs template now carries windows_subsystem, but the source (host binary, packager, or a helper) was not isolated for existing apps. Phase 2 checks the PE subsystem of the new host. | 2 |
| regression | The rustx-engine tests directory holds 30+ files, probably opening a real CEF window through tests/common. Not read; phase 1 lists them. | 1, 7 |
| runtime | Patch derives only Debug, Clone and PartialEq and rustx-dom declares no serde dependency, so serialization is new work. | 1 |
| runtime | Page loading uses file:/// URLs and a file origin workaround in cef_app.rs. It becomes a custom protocol. | 2 |
| documentation | README files and the design document describe CEF and in-process natives. | 7 |
| runtime | Only Windows can be verified on this machine. macOS and Linux are verified in CI only, or not verified. | 2, 6, 7 |

## Rules

| Rule   | Why it applies |
| ------ | -------------- |
| BUG.md: the advisor orients fixes and debugging, only advisor-provided fixes are applied | Governs how each phase is executed. |
| BUG.md: use aidd-dev and ponytail skills | Governs how each phase is executed. |
| Bash backtick commit corruption memory: write commit messages to a file and use -F | Applies to any commit made during the migration. |
| panic = "abort" is excluded | panics::contain relies on unwinding, an aborting handler would kill the application. |

## Decisions

| Decision   | Why |
| ---------- | --- |
| Target wry and tao, with the Rust VDOM staying in the host process and patches sent over IPC | The model Dioxus desktop uses; the Patch type already exists in rustx-dom. |
| Write the new engine in a separate crate rustx-webview and select it through a cargo feature on the rustx facade, with no generic trait | CEF stays working until parity, so every phase ships and verifies alone. |
| External navigation is supported: links open in the system browser, an application-configured allow-list may load inside the window, and IPC answers only to the application's own origin | Requested by the user. CEF refused all remote content because its render process ran Rust unsandboxed (guard.rs); with a system webview the Rust code is in the host process, so a remote page reaches only the origin-checked IPC. The allow-list default and the system-browser default are assumptions to confirm. |
| Native functions become Promise based | A system webview offers only asynchronous IPC. Breaking change, documented in phase 4. |
| Packaging without external tools, cross-compile, fast refresh and LSP parity are separate plans | Each is large and independent; this plan only removes the CEF payload they all depend on. |
