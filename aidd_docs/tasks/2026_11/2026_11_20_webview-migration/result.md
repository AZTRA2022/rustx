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

- A native call is a synchronous request to the host: the page waits, and a slow native freezes it.
- A callback's return value does not cross to the JavaScript library.
- `text`, `attribute`, `tag_name`, `query_selector` and `count_matching` answer a typed `DomError` instead of reading the page.
- A second window is another process of the same executable, so it has its own state, as on CEF.
- `rustx-engine` no longer exists. The facade still exports the webview crate as `rustx_engine`, because the compiler's generated code writes that path.
- The 30 integration test files of the old engine went with it. What they covered is carried by the unit tests of `rustx-webview` and the application suites above; no one-to-one port was done.

## Documentation still naming Chromium

README, the crate READMEs, installation, packaging, where-code-runs and the JavaScript-library pages were rewritten. These pages of `site/content/docs` still name CEF or Chromium and need a page-by-page pass: getting-started/{assets, migrating, project-structure}, reference/index, the-system/{app-data, devices, dropped-files, file-dialogs, index, menus, platforms, printing, screens-and-capture, security}, the-window/closing, tooling/{configuration, contributing, debugging, environment, testing, the-commands, the-packages, troubleshooting, upgrading}, whats-left, and the READMEs of rustx-dom, rustx-macros, rustx-os. `rustx_os::print` and the window-printing API of the old engine were not carried over.
