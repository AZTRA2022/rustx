# Follow-ups found while implementing

From the review of phase 1, to be handled where they are named.

## Phase 3

Settled by the remote DOM (first slice, committed): point 1 (handler slots stay in Rust and the table of listeners holds weak slots, swept when it has doubled), point 2 (the handler is cloned out of its slot before it is called and a render is deferred to the next tick), point 4 (the page never sees a path: the engine's own `render.rs` and `tree.rs` do the numbering). Still open: point 3, and a test that 100 renders leave the listener table constant.

1. `Patch::ReplaceHandlers` is emitted by `diff` on every render of every tag that has a handler, and `lower` gives it a fresh id each time without freeing the old one. The registry grows without bound and an old id keeps running a stale closure. `session.rs` must hold the path to id mirror: `Registry::replace(id, handlers)` keeps the id stable, `Replace` and `Remove` forget the ids of the removed subtree recursively. Test: 100 renders, constant registry size.
2. `Registry::dispatch` and `call_back` call user code while holding `&self`. If a handler triggers a synchronous render and the registry sits in a `RefCell`, it panics with "already borrowed". Expose a lookup that returns the `Handler` or `Callback`, and call it once the borrow is released, or prove that rendering is always deferred to the next frame.
3. The click-to-paint measure promised in `baseline.md` must also be taken on the CEF counter, with the page script written in this phase, before CEF is removed in phase 7. Complete `baseline.md` then.
4. A path resolved on the page must follow `diff`'s numbering, where a `Fragment` takes a level and `Empty` exists while neither has a DOM node. Read how `render.rs` and `mount.rs` map a `Path` to the DOM today and reproduce it exactly.

## Architecture change decided during phase 3

The plan's phase 3 applies patches in JavaScript and lowers `Patch` in `rustx-webview`. Reading `render.rs`, `tree.rs` and `router.rs` showed that the engine's render pipeline touches the page only through `Document` and `Node` in `dom.rs`. The engine therefore keeps `render.rs`, `tree.rs` and the invariants they hold, and gets a second backend: `Node` and `Document` become a remote DOM, a batch of commands per frame sent to a small JavaScript interpreter, with node ids. The two backends coexist until phase 7, chosen once at start through `RUSTX_ENGINE=webview`. Acceptance criteria do not change. Consequences: the `Patch` lowering and `Registry` of phase 1 become dead code and are removed in the first commit of the new backend; points 1, 2 and 4 above are settled by design (handler slots stay in Rust, a render is deferred to the next tick, paths never reach the page); controlled inputs need a sequence number so a stale value never overwrites typing.

## Phase 2 measures

Release probe, Windows: 0.91 MB executable, PE subsystem 2 (no console), window handle present after 59 to 249 ms, the page's script ran and reached the host by IPC after 1.0 to 1.5 s. A window handle is not a painted page. A missing WebView2 runtime exits with code 1 and a message naming the runtime. `file://` is blocked by the browser itself before the navigation handler is called, so its refusal is not proved by the host test and stays covered by the policy's unit test. The same test uses `ftp://`, which does reach the handler.

## Phase 2

- `guard.rs` is not touched in this phase: CEF stays active until phase 7 and its render process is still unsandboxed, so relaxing the rule now would be a security regression. The navigation policy is written in `rustx-webview`, and `guard.rs` leaves with CEF. This differs from the phase 2 file tree, which lists `guard.rs` as modified.

## Phases 3 to 5 closed

Delivered as a remote DOM (ops with node ids, one batch per frame) instead of `applier.js` patches; natives are a synchronous XHR to the host's custom protocol, answered `{ok}` or `{err}`; the React boundary passes props as JSON with `{"__rustx_cb": id}` for functions, ids stable per place. Verified: counter 4/4, hybrid 9/9 on the webview engine. Known limits: a callback's return value does not cross; a slow native freezes the page; `text`, `attribute`, `tag_name`, `query_selector` and `count_matching` answer a typed `DomError` on this engine. Still open: click-to-paint on both engines (`baseline.md`).

## Phase 6 closed

Controls are orders carried out on tao's window (not `Pane::from_handle`): tested on Windows (fullscreen, restore, close); macOS and Linux are the same tao calls, not run here. Dropped files come from wry's drag-drop handler with real paths (unit-tested, not driven by a real drop). A second window is a child process of the same executable (`--rustx-window=<json>`), as every window was a render process on CEF; it ends when its stdin pipe closes, so it goes with its parent or `Window::close`. `rustx_os::blocking::draws_here()` is called by the host. `on_frame` in video-editor and a manual run of monitor's second window are still to be observed.

## Slow natives

A synchronous native froze the page and the window loop. `register_async_native` is the answer: the closure starts the work on the window thread, the returned `Send` future runs on tokio, and the result reaches the page as a promise through a script sent by the loop. Panics and errors reject. Synchronous natives stay, because the engine's own (`rustx_scenario`, `rustx_report`, `rustx_about`) and the pages' scripts read them as plain values.
