# rustx-webview

**The host of a rustx window: the system's web view (WebView2, WKWebView,
WebKitGTK through wry and tao), the page interpreter, the frame loop, the
router and the tasks.** An internal crate of
[rustx](https://github.com/AZTRA2022/rustx).

> **You probably want the `rustx` crate instead** — the package `rustx-app`,
> which re-exports this one as `rustx::window`, `rustx::router` and the rest.

What it owns:

- the remote DOM: Rust writes operations, one batch per frame, and the page's
  small interpreter applies them and reports events back;
- natives: synchronous (`register_native`) and asynchronous
  (`register_async_native`) calls from the page into Rust;
- `file://` content served safely to the page (`files`);
- the window: size, position, limits, focus, visibility, events, a close guard,
  remembered state (`window`);
- the application's data and tests: the `probe` binary drives a real window.

Apache-2.0 OR MIT.
