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
| 7 | Browser defaults | F5, Ctrl+R, Ctrl+P, Ctrl+wheel, context menu | ignored in a shipped app | all active | WebView2 defaults | accelerator keys, zoom, context menus and swipe navigation off unless the devtools are on | hand (a synthetic event does not trigger them) |
| 8 | Fields | `<select value="b">`, `<input type=range value=150 max=200>` | `b`, `150` | `a`, `100` | `value` was set before the options and `max` existed | properties are set after the children and the other attributes | auto: `a_field_written_before…` |
