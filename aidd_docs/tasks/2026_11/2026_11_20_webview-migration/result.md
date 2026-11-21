# Result: apps/counter, CEF against the system webview

Measured on the same machine as `baseline.md` (Windows 10 Pro 10.0.19045), same commands: `rustx package --no-zip --out <dir>` in `apps/counter`, and the same stopwatch from `Start-Process` to a window handle.

| Measure | CEF (baseline) | Webview | Change |
| --- | --- | --- | --- |
| Release build and packaging, cold | 1757 s | 159 s | about 11 times faster |
| Delivered folder, without installer | 411 MB, 237 files | 1.76 MB, 3 files (`counter.exe`, `assets/`, manifest) | about 230 times smaller |
| Installer (Inno Setup) | 132.4 MB | 2.48 MB | about 53 times smaller |
| `counter.exe` | 1.14 MB | 1.84 MB | the webview host is in it |
| Time to window handle, 3 runs | 348 / 118 / 136 ms | 472 / 112 / 116 ms | the same: it is the window, not a painted page |

The cold build of the baseline was measured while another crate compiled, so 1757 s is an upper bound; the 159 s run reused the dependencies already compiled for the earlier test runs, so it is a lower bound. Neither is a clean figure. The order of magnitude is the finding.

## What is not measured

- **Click-to-paint latency.** The baseline could not take it on CEF and CEF is now removed, so there is no comparison. On the webview, a click reaches the page's handler, the Rust closure runs, and the frame's commands go back in one `evaluate_script`; the tests of the three engines' shared behaviour pass in about 2 s for a whole suite, which bounds it loosely and measures nothing.
- **The slowness BUG.md reports** (CEF's GPU process failing and falling back to software rendering). It can no longer be reproduced: Chromium is gone. Whether the new engine is fast on that machine is judged by use, not by a number here.
- **Rebuild time after editing a page.**
- **macOS and Linux.** The code is tao and wry on both and the package has a `.deb` dependency on `libwebkit2gtk-4.1-0`, but none of it was built or run here.

## What was verified on the webview engine (Windows)

counter 4/4, hybrid 9/9, monitor 6/6, capture 7/7, zoo 6/6, catalogue 3/3, the rustx-webview crate's own tests, rustx-cli 33 unit tests and its integration tests. Window controls (fullscreen, restore, close) and a second window with its route in the URL fragment. Not driven: a real file drop (the paths come from wry's handler and are unit-tested), `video-editor` (only its 19 unit tests ran), and the apps without tests (agent-manager, data-intel, ops-monitor, utility-hub, vault-storage, multitask-hub, nemo-design).

## Changes of behaviour a user of the framework should know

- `register_native` is a synchronous request to the host: the page waits, and a slow one freezes it. `register_async_native` answers with a promise and runs its future on the async runtime, so the window stays free (tested with a 1.5 s native while other natives and timers keep running).
- A callback's return value does not cross to the JavaScript library.
- `text`, `attribute`, `tag_name`, `query_selector` and `count_matching` answer a typed `DomError` instead of reading the page.
- A second window is another process of the same executable, so it has its own state, as on CEF.
- `rustx-engine` no longer exists. The facade still exports the webview crate as `rustx_engine`, because the compiler's generated code writes that path.
- The 30 integration test files of the old engine went with it. What they covered is carried by the unit tests of `rustx-webview` and the application suites above; no one-to-one port was done.

## Documentation

Every page of `site/content/docs`, the READMEs and the video-editor comments that named CEF or Chromium were rewritten (2026-11-21); what still says Chromium is the Electron column of the migration table and two paragraphs of `whats-left.mdx` marked as written in the CEF era. `printing.mdx` is honest: print through the system dialog, no print-to-PDF.

## Closed after the first result (2026-11-21)

- **Seven apps smoke-run** (agent-manager, data-intel, ops-monitor, utility-hub, vault-storage, multitask-hub, nemo-design): all start. Found and fixed: an app with no `assets/` failed to start; two apps read the network state and the secret store on the drawing thread. Open: a click handler cannot start a `task` (ledger row 22).
- **Setup without Inno Setup**: `rustx-setup.exe` stub plus appended payload, 1.9 MB for counter (stub 1.1 MB); silent install, launch and uninstall run on Windows with the profile redirected to a temp folder. `RUSTX_INSTALLER=inno` keeps the old route.
- **Debug build**: `debug = "line-tables-only"`; counter from clean, 195 s and 0.84 GB for `target/debug` (before, the whole workspace's debug folder was 22 GB).
- **Bundle**: `"rustxNoTags": [...]` in package.json keeps a library out of the page's tags (a first version that guessed from the pages' text broke zoo, whose pages write bare tags like `<Markdown>`; CI found it); video-editor 7.2 MB to 1.6 MB with Phosphor listed.
- **Fast refresh, partial**: a saved asset reloads the page in place (Rust state kept); a `.rsx` edit still rebuilds and relaunches with named state carried.
- **Context menu**: the system menu is kept for text; the page cancels it elsewhere. Hand check owed: right-click on text and Ctrl+C/V/X/A in a release build.
- **CI**: `.github/workflows/ci.yml` rewritten for the webview (WebKitGTK packages, Xvfb tests on Linux, silent install check on Windows). Written, not pushed, not run on a runner.
- **Feature inventory**: `feature-inventory.md`.

## Still open

Click-to-paint number; edit-to-screen number for a `.rsx` edit; macOS and Linux only on CI runners (never a real desktop; the Mac runner's window sizing numbers are in this file); native hot patch (true fast refresh); window state persistence; multi-window as threads; the context-menu hand check; the parked probes P2 to P11 in `bug-hunt.md`.

## GitHub CI (2026-11-21, draft PR 7)

Run 44887010399 (the latest; the first fully green one was 44874892110): all eight jobs green. What that means:

- **Windows**: fmt, clippy, the whole workspace's tests (1,180+), docs, packaging; the counter app is packaged, the setup is installed silently and checked, then the CI step ends.
- **Linux**: `rustx-os` tests; the whole `rustx-webview` suite (page, natives, files, window API) under Xvfb with WebKitGTK; a `.deb` that `dpkg` installs and `desktop-file-validate` accepts.
- **macOS**: `rustx-os` tests, the whole webview suite on a hosted Mac, the `.pkg` (with the project's screens through `productbuild`) and `.dmg`, installed and started.

What CI found that a Windows-only loop never would: WebKitGTK gives a custom protocol no request body (natives now carry small arguments in the address); `img.src = "file://…"` starts a load the origin refuses before the observer runs (the property is now rewritten where it is set); a Linux runner's Xvfb has no window manager (fullscreen untestable there); Git Bash rewrites `/S` into a path (the setup also takes `--silent`).

Not covered by CI: a real desktop on either system (tray, dialogs answered by a person, cameras, fullscreen), `confirm_close` (no way to synthesise a close request), the installer pages on macOS beyond `pkgutil --expand` finding them, the Windows setup's graphical screens by eye.

Limit to know: a synchronous native with arguments over 200 KB sends them in the body, which WebKitGTK drops; use an asynchronous native for big payloads.

macOS webview (CI, hosted Mac): the size set from Rust was applied (the page saw 777x555, the outer size followed, a Resized event arrived) but tao's `inner_size()` kept returning the old one, so `window::state()` was stale there. When only the outer size moved, the published inner size moves with it (a correction in the host's snapshot); the two tests that had been skipped on macOS run there again.

Since then CI also proves that the installed application **starts and stays up for eight seconds** on all three systems (Windows `counter.exe`, Linux under Xvfb, macOS `Counter.app` after `installer -pkg`), that the Windows silent uninstaller removes the folder, and the macOS job runs the whole webview suite (22 tests, none skipped) and the installed `Counter.app` is started from `/Applications` (the `.pkg` is made non-relocatable, otherwise the installer updated a copy found in `dist/`).

## Measures (Windows, debug build, this machine)

- Page ready after 0.8 to 1.1 s from the process start (`RUSTX_TRACE=1` prints it, the clock starting at the first statement of `main`); a window hidden until then is no slower than a visible one (`RUSTX_SHOW_NOW=1`), and the page, not the 1.2 s fallback, is what shows it.
- `.rsx` edit to a new window with the page ready under `rustx dev`: 5.4 s for a comment-only change on counter (incremental cargo build included). An asset edit reloads the page in place without a build (not timed).
- Installed size: setup 1.9 MB (stub 1.1 MB), folder 2.1 MB for counter.
- **Release build, Windows**: page ready 0.81 to 0.96 s after the first statement of `main` (counter, three runs) and 0.86 to 0.89 s (video-editor); executables 2.09 MB and 4.99 MB. Most of it is WebView2 creating its environment; the framework's own part (assets, window, first frame) is small beside it.
- **Click to paint**: from `click()` to the frame that shows the host's answer, median 9 ms and worst 19 ms over 15 clicks (Windows, probe window). The probe test `click_to_paint_is_measured_and_not_slow` runs everywhere and bounds the median at 500 ms.
