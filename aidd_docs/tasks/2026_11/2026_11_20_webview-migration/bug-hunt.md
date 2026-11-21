# Bug hunt: video-editor as a carrier for framework bugs

One row per probe. A probe is the smallest feature or test that exercises a surface; a fix goes in the framework, with a regression test, and is committed with it.

Evidence column: **auto** = an automated test fails without the fix and passes with it; **hand** = built, only driven by hand; **n/v** = not verifiable on this machine (steps for the user in the last section).

| # | Surface | Probe | Expected | Observed | Root cause | Fix | Evidence |
|---|---|---|---|---|---|---|---|
| 1 | URLs | `<video src="file:///…">` from Rust | the clip plays | black monitor, nothing said | a page of the app origin may not load `file://` | interpreter rewrites `file://` (attributes, inline CSS, fetch, XHR, library-made DOM, scripts running before load) to a host route with byte ranges | auto: `a_page_that_writes_file_urls…`, `a_video_from_a_file_url…` (mp4, webm, mkv, mov decode, play with no gesture, seek) |
| 2 | URLs | file larger than 4 MiB through `fetch` | whole file | 206 with the first 4 MiB, silently truncated | the route chunked answers without a `Range` header | whole file as 200 up to 256 MiB | auto: `files::tests::a_large_file…` |
| 3 | Security | markup injected into a page reads `file:///` | refused | any absolute path was served | the route trusted every same-site request | grants: a path is served only if Rust wrote it into the page, `files::allow`, `files::url`, or a dropped file; cross-site requests and `..` refused | auto: `a_file_nobody_granted_cannot_be_read_by_injected_markup` |
| 4 | URLs | names with `#`, `%`, `é`, `&`, `[]`, spaces | served | `#` cut the name, `%` double-decoded | a hand-written `file://` is not a URL | `files::url(path)` encodes per segment; the interpreter encodes per segment | auto: `files::tests::url_encodes_awkward_names…` |
| 5 | Events | timeline scrub, wheel zoom, context menu | pointer position, button, wheel, size of the element | `Event` had none of them, and `id` was the inner element | `record()` sent only value, id, key, checked, modifiers | `x`, `y`, `local_x`, `local_y`, `width`, `height`, `button`, `delta_x`, `delta_y`, `current_id` | auto: `a_pointer_event_says_where…` |
| 6 | Browser defaults | `<form>` submit | handlers run, page stays | the page navigated, replacing the document the host holds ids for | a submit navigates by default and Rust cannot `preventDefault` after a round trip | the interpreter cancels every submit of the app's pages | auto: `a_form_submit_and_a_reload…` |
| 7 | Browser defaults | F5, Ctrl+R, Ctrl+P, Ctrl+wheel, context menu | ignored in a shipped app | all active | WebView2 defaults | accelerator keys, zoom, context menus and swipe navigation off unless the devtools are on | n/v: a synthetic event does not trigger them. User: press F5, Ctrl+P, Ctrl+wheel and right-click in a release build; nothing should happen. Side effect to decide: no cut/copy/paste menu in text fields |
| 8 | Fields | `<select value="b">`, `<input type=range value=150 max=200>` | `b`, `150` | `a`, `100` | `value` was set before the options and `max` existed | properties are set after the children and the other attributes | auto: `a_field_written_before…` |
| 9 | Trust | a path inside text Rust wrote (`title`, a clip name, a field value) | text | became readable by the page | the first grant ran on every attribute and property | grants only where the interpreter rewrites: `src`, `href`, `poster`, `data`, `url()` in `style` | auto: `a_path_in_text_rust_wrote_does_not_become_readable`, `only_the_attributes_that_load_something_grant_a_path` |
| 10 | Lifecycle | `location.reload()` after two clicks | the counter is back and answers | blank page, `no #inc` | nothing mounted again when the new document said ready | the render keeps what the application mounted (not React's own roots) and mounts it again on a ready that follows a gone page | auto: `after_a_reload_the_counter_is_drawn_again…` |
| 11 | Events | 20 wheel/move events in a frame | one message per frame | one message and one render each | every event was posted | coalesced per node and kind to one per animation frame; wheel deltas added; anything else flushes the queue first | auto: `a_burst_of_wheel_and_move_events…` |
| 12 | Regression | `rustx-dom` test builds an `Event` literal | compiles | did not compile after #5 | new fields | test updated | auto (found by the full run) |
| 13 | Build | first run of a suite after a build (zoo twice, hybrid, video-editor 3/3) | green | every test "never reported"; green after a rebuild | observed: zoo's `assets/rustx-bundle.js` was **0 bytes** after the failing run. The attribute expands for the binary and its test target at the same time and both ran esbuild onto one `bundle.js`; one read it while the other had truncated it and copied nothing into the assets | a cross-process lock around the bundle build | partly: three forced rebuilds of zoo left a full bundle; the race cannot be forced, so watch for a recurrence |
| 14 | Start | an application opened right after the previous run closed | opens | `WebView2 error 0x800700AA: the requested resource is in use`, no window (seen once, with the full `Error: EngineInit(...)` line in the report) | the old browser process still holds the profile folder for a moment | the host retries the build every 250 ms for up to 10 s on that error | auto, partly: the sequential tests of `video-editor/tests/app.rs` no longer hit it. This does **not** explain row 13's later 3/3 "never reported" after the grade build: no EngineInit line was shown that time and a rebuild cured it. Row 13 stays open; next time record the unfiltered report, the `msedgewebview2` processes and `rustx-bundle.js` size/mtime before rebuilding |
| 15 | Fields | Radix `Slider` controlled from Rust (value `[x]`, `onValueChange` with a list) | the grade follows | worked | (no bug: a library's prop is a `Value`, annotated `|given: rustx::Value|`) | none; `Event::from_value` now also reads a list into `numbers` | auto: `a_radix_slider_owned_by_rust_moves_the_grade` |
| 16 | Fields | four slider steps before Rust answers (4 ms apart) | 0.20 | 0.20 | hypothesis refuted: the round trip keeps up at that speed; four steps in one JS task collapse in plain React too | an optimistic-value layer in the React bridge was written, found to change nothing, and removed | auto: `a_burst_of_slider_steps…` (guard) |
| 17 | Monitor | CSS grade on the `<video>` | `filter: brightness()…` | empty `style` | the page set the style on one of the two `<video>` branches | app fix | auto: `a_radix_slider_owned_by_rust_moves_the_grade` |
| 18 | Callbacks | a Radix `Switch` per row of a list of clips, rows keyed by clip, then a clip moved | the switch of a row acts on that row's clip | it acted on the other clip | the diff compares a React boundary without its callbacks, so a render that only changed a closure (a captured index) sent no patch and the page kept calling the old closure | a boundary with callbacks is always handed over again; the host sends `fm` to the page only when the description differs | auto: `a_switch_acts_on_the_clip_its_row_names…`, `a_node_with_callbacks_is_handed_over_again…` |
| 19 | Callbacks | a boundary with no callback of its own that holds children with callbacks (a `Card` of controls capturing a clip index) | closures replaced | stale when nothing on screen changed | the first fix of #18 looked at the top node only | `carries_callbacks` is recursive (children, nested boundaries, tags with handlers) | auto: `a_callback_anywhere_inside_a_boundary_hands_it_over_again` |

## Parked probes (open, not run)

| # | Surface | Probe | Why it matters |
|---|---|---|---|
| P1 | SVG | `<svg>`, `<path>`, `<rect>` written in `.rsx`: namespace and `getBBox()` | the interpreter uses `createElement` with no namespace; curves, waveforms, masks, keyframes depend on it |
| P2 | Files | subtitles from `.txt`, `.md`, `.srt`: BOM, CRLF, UTF-16, empty, injection with `file:`, `javascript:`, raw HTML | encodings and the grant model |
| P3 | Lifecycle | project save and load, flush on close | see Tier 1 below |
| P4 | Fields | `<select>` whose options arrive later with an unchanged value; `max` and `value` changed in one render | the diff-path version of row 8 |
| P5 | Fields | a shadcn `Input` controlled from Rust, typed in bursts | React puts a controlled input back before Rust answers |
| P6 | Animation | `AnimatePresence` exit animation | `fu` runs before the node leaves |
| P7 | Async | import A then B quickly; cancel on route change | races |
| P8 | Web | `getUserMedia`, CORS `fetch` from the custom origin | devices |
| P9 | Size | ~~`rustx-bundle.js` is 7.2 MB with Phosphor imported as a namespace~~ done: `rustxNoTags` in package.json (1.6 MB) | size and start time |
| P10 | DnD | HTML5 drag to reorder clips with wry's drop handler on | needs a real drag |
| P11 | AE | about fifty After Effects-style features | app-only unless they hit SVG, canvas, contenteditable/IME or high-frequency playback |
| W | Build | row 13: watch for a recurrence of an empty bundle | |

## Smoke run of the seven untested apps (2026-11-21)

Each built and launched for 10 s from its folder. Stderr is the evidence: the
interpreter forwards `error` and `unhandledrejection` from the page to it.

| # | App | Found | Fix |
|---|---|---|---|
| 20 | multitask-hub | `IndexNotFound(assets)`: an app with no JavaScript has no `assets/`, and `App::new` demanded the folder even when the framework serves its own document | `App::new` needs the folder only when it has no document; unit test `an_app_with_no_assets_folder_runs_on_its_own_document`. `rustx package` already treats a missing `assets/` as nothing to copy |
| 21 | ops-monitor, vault-storage | network state and secret store read on the drawing thread (the guard warned) | both moved into a component-level `task(async move { … })`; stderr empty afterwards |
| 22 | vault-storage | the same guard fires for the remember/forget buttons | **open, framework**: `task` exists only at component level (it is a render-time cell), so a click handler cannot start a future that writes state. A handler-started task is the missing piece of "advanced async" |
| 23 | all | right-click showed no cut/copy/paste in release (context menus tied to devtools) | host keeps the system menu; the page cancels `contextmenu` off editable targets and selections, unless the page cancelled it first or devtools are on. Hand check still owed: right-click on text, Ctrl+C/V/X/A in a release build |
| – | build | disk reached 0.4 GB free (target 25 GB): cargo died with os error 112, the smoke script reported it as a build failure | cleaned incremental/release; Bash's `link` shadows MSVC's, build from PowerShell |

Parity audit with the deleted engine: 30 old test files listed; render, frame,
lifecycle and posted-update promises ported to the probe (`engine` scenario);
no app reads the DOM synchronously; SVG was never supported by the old DOM (not
a regression); `print_to_pdf` dropped (the system webviews cannot), docs updated.
