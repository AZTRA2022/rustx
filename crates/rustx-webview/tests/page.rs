//! Components drawn in a real window, through the commands of a frame.
//!
//! Each test mounts one of the probe's scenarios under `#app` and drives
//! it from the page, the way a user would, then reads what the page ended
//! up showing. The engine's own scenarios on CEF (`hello`) are the model;
//! these are the ones that carry over to the webview.

mod common;

use common::{folder, probe};

/// The page side of a scenario: waits for an element, with a helper to
/// pause, and posts what it sees.
const HELPERS: &str = "
  const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
  const wait = async (id) => {
    for (let i = 0; i < 200; i++) {
      const found = document.getElementById(id);
      if (found) return found;
      await sleep(25);
    }
    throw new Error('no #' + id);
  };
  const say = (text) => window.ipc.postMessage(text);
";

fn page(script: &str) -> String {
    format!("<!doctype html><div id=\"app\"></div><script>{HELPERS}(async () => {{ {script} }})();</script>")
}

#[test]
fn a_counter_draws_its_first_frame_and_each_click_after_it() {
    let root = folder(
        "counter",
        &page(
            "const button = await wait('inc');
             say('first=' + button.textContent);
             for (let i = 0; i < 3; i++) { button.click(); await sleep(250); }
             say('count=' + document.getElementById('inc').textContent);
             say('fixed=' + document.getElementById('fixed').textContent);",
        ),
    );
    let run = probe(&root, &["--scenario", "counter", "--seconds", "6"], &[]);
    assert_eq!(
        run.messages,
        vec!["first=0", "count=3", "fixed=fixed"],
        "{}",
        run.stderr
    );
    assert!(run.stderr.is_empty(), "stderr: {}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_burst_of_typing_faster_than_the_round_trip_loses_no_keystroke() {
    // The host takes 40 ms per input and the page types every 5 ms, so
    // every value the host sends back is older than what is in the
    // field, and must not overwrite it. Each keystroke is appended to
    // what the field *holds*, as a keyboard does: an overwrite is carried
    // into every letter typed after it, and the text comes out short.
    let root = folder(
        "echo",
        &page(
            "const field = await wait('field');
             field.focus();
             let typed = '';
             for (const letter of 'the quick brown fox jumps over') {
               typed += letter;
               field.value = field.value + letter;
               field.dispatchEvent(new Event('input', { bubbles: true }));
               await sleep(5);
             }
             await sleep(2500);
             say('typed=' + typed);
             say('field=' + field.value);
             say('echo=' + document.getElementById('echo').textContent);",
        ),
    );
    let delay = std::path::Path::new("40");
    let run = probe(
        &root,
        &["--scenario", "echo", "--seconds", "10"],
        &[("PROBE_INPUT_DELAY_MS", delay)],
    );
    let expected = "the quick brown fox jumps over";
    assert_eq!(
        run.messages,
        vec![
            format!("typed={expected}"),
            format!("field={expected}"),
            format!("echo={expected}")
        ],
        "{}",
        run.stderr
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_keyed_list_rotates_with_the_text_before_it_left_in_place() {
    // Moves, in a list that has a text node among its siblings: the case
    // the CEF engine's `place` and `first_attached` were written for.
    let root = folder(
        "keyed",
        &page(
            "const rotate = await wait('rotate');
             const order = () => Array.from(document.querySelectorAll('#list li'), (li) => li.id).join(',');
             const head = () => document.getElementById('list').firstChild.textContent;
             say('start=' + order() + ' head=' + head());
             rotate.click(); await sleep(300);
             say('once=' + order() + ' head=' + head());
             rotate.click(); await sleep(300);
             say('twice=' + order() + ' head=' + head());",
        ),
    );
    let run = probe(&root, &["--scenario", "keyed", "--seconds", "6"], &[]);
    assert_eq!(
        run.messages,
        vec![
            "start=row-1,row-2,row-3,row-4 head=Items:",
            "once=row-2,row-3,row-4,row-1 head=Items:",
            "twice=row-3,row-4,row-1,row-2 head=Items:",
        ],
        "{}",
        run.stderr
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_native_called_from_the_page_resolves_or_rejects_and_the_window_carries_on() {
    let root = folder(
        "natives",
        &page(
            "const outcome = async (call) => { try { return 'ok:' + await call(); } catch (e) { return 'err:' + e.message; } };
             say(await outcome(() => add(2, 3)));
             say(await outcome(() => fails()));
             say(await outcome(() => panics()));
             say(await outcome(() => add(1, 1)));
             say(await outcome(() => add('x')));",
        ),
    );
    let run = probe(&root, &["--scenario", "natives", "--seconds", "5"], &[]);
    assert_eq!(
        run.messages,
        vec![
            "ok:5",
            "err:it failed",
            "err:`panics` panicked",
            "ok:2",
            "err:add takes two numbers"
        ],
        "{}",
        run.stderr
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_window_follows_the_orders_the_page_gives_it() {
    let root = folder(
        "controls",
        &page(
            "const width = () => window.innerWidth;
             const settle = async () => { await sleep(1200); return width(); };
             await sleep(800);
             const normal = width();
             order('fullscreen');
             const full = await settle();
             say('fullscreen_wider=' + (full > normal));
             order('restore');
             say('restored=' + ((await settle()) === normal));
             order('close');
             await sleep(8000);
             say('still_open=yes');",
        ),
    );
    let run = probe(&root, &["--scenario", "controls", "--seconds", "20"], &[]);
    assert_eq!(
        run.messages,
        vec!["fullscreen_wider=true", "restored=true"],
        "close must end the loop before the page's last message: {}",
        run.stderr
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_slow_asynchronous_native_leaves_the_page_and_the_window_free() {
    let root = folder(
        "slow",
        &page(
            "const started = performance.now();
             const slow = window.slow(42);
             // While it runs, the host still answers other natives and the
             // page still runs its own timers.
             let ticks = 0;
             const timer = setInterval(() => ticks++, 50);
             for (let i = 0; i < 3; i++) await sleep(100), window.add(i, 1);
             const freeAfter = performance.now() - started;
             say('free_while_pending=' + (freeAfter < 1000));
             say('ticks_ran=' + (ticks >= 3));
             say('slow=' + (await slow));
             clearInterval(timer);
             say('waited=' + (performance.now() - started >= 1400));
             try { await window.slow_fails(); } catch (error) { say('rejected=' + error.message); }
             try { await window.slow_panics(); } catch (error) { say('panic_rejected=yes'); }
             say('still_alive=' + window.add(1, 2));",
        ),
    );
    let run = probe(&root, &["--scenario", "natives", "--seconds", "8"], &[]);
    assert_eq!(
        run.messages,
        vec![
            "free_while_pending=true",
            "ticks_ran=true",
            "slow=42",
            "waited=true",
            "rejected=slow failure",
            "panic_rejected=yes",
            "still_alive=3",
        ],
        "{}",
        run.stderr
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_page_that_writes_file_urls_gets_the_files() {
    // A 1x1 PNG and a text file on disk, outside the page's folder.
    let place = std::env::temp_dir().join(format!("rustx-fileurl-{}", std::process::id()));
    std::fs::create_dir_all(&place).expect("a folder");
    let png: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21,
        196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 255, 255, 63, 0, 5, 254, 2, 254, 167, 53, 129, 132,
        0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    std::fs::write(place.join("a b.png"), png).expect("a png");
    std::fs::write(place.join("note.txt"), "hello from disk").expect("a note");
    let url = |name: &str| format!("file:///{}/{}", place.display().to_string().replace(std::path::MAIN_SEPARATOR, "/").trim_start_matches('/'), name);
    let (image, note) = (url("a%20b.png"), url("note.txt"));
    let root = folder(
        "fileurl",
        &page(&format!(
            "const img = document.createElement('img');
             img.src = '{image}';
             document.body.appendChild(img);
             for (let i = 0; i < 80 && !img.naturalWidth; i++) await sleep(50);
             say('image=' + img.naturalWidth);
             say('fetched=' + await (await fetch('{note}')).text());
             const late = new Image(); document.body.appendChild(late);
             late.setAttribute('src', '{image}');
             for (let i = 0; i < 80 && !late.naturalWidth; i++) await sleep(50);
             say('later=' + late.naturalWidth);"
        )),
    );
    std::fs::write(place.join("..secret-name.txt"), "x").ok();
    let run = probe(&root, &["--scenario", "counter", "--seconds", "8", "--files", &place.display().to_string()], &[]);
    assert_eq!(run.messages, vec!["image=1", "fetched=hello from disk", "later=1"], "{}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(place);
}

/// A clip made by ffmpeg in the given container, or `None` where ffmpeg is not installed.
fn clip(place: &std::path::Path, name: &str, codecs: &[&str]) -> Option<String> {
    let file = place.join(name);
    let made = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc=size=320x240:rate=25:duration=3"])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=3"])
        .args(codecs)
        .arg(&file)
        .status()
        .ok()?;
    made.success().then(|| {
        format!("file:///{}", file.display().to_string().replace(std::path::MAIN_SEPARATOR, "/").trim_start_matches('/'))
    })
}

#[test]
fn a_video_from_a_file_url_decodes_plays_without_a_gesture_and_seeks() {
    let place = std::env::temp_dir().join(format!("rustx-video-{}", std::process::id()));
    std::fs::create_dir_all(&place).expect("a folder");
    let formats = [
        ("h264.mp4", vec!["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"]),
        ("vp9.webm", vec!["-c:v", "libvpx-vp9", "-c:a", "libopus"]),
        ("h264.mkv", vec!["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"]),
        ("h264.mov", vec!["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"]),
    ];
    let mut script = String::new();
    let mut expected = Vec::new();
    for (name, codecs) in &formats {
        let Some(url) = clip(&place, name, codecs) else {
            eprintln!("ffmpeg is not installed: video test skipped");
            let _ = std::fs::remove_dir_all(&place);
            return;
        };
        script.push_str(&format!(
            "{{ const v = document.createElement('video'); v.src = '{url}'; document.body.appendChild(v);
               let ok = true;
               for (let i = 0; i < 100 && v.readyState < 2 && !v.error; i++) await sleep(50);
               const decoded = v.readyState >= 2 && v.videoWidth === 320 && Math.abs(v.duration - 3) < 0.3;
               let played = 'no';
               try {{ await v.play(); await sleep(700); played = v.currentTime > 0.3 ? 'yes' : 'stuck'; }} catch (e) {{ played = e.name; }}
               v.currentTime = 2; await sleep(500);
               const seeked = v.currentTime >= 1.9;
               say('{name}=' + decoded + '/' + played + '/' + seeked); }}\n"
        ));
        expected.push(format!("{name}=true/yes/true"));
    }
    let root = folder("video", &page(&script));
    let run = probe(&root, &["--scenario", "counter", "--seconds", "30", "--files", &place.display().to_string()], &[]);
    // What this engine cannot decode is a finding, printed whole.
    eprintln!("VIDEO RESULTS: {:?}", run.messages);
    assert_eq!(run.messages, expected, "{}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(place);
}

#[test]
fn a_file_nobody_granted_cannot_be_read_by_injected_markup() {
    let place = std::env::temp_dir().join(format!("rustx-secret-{}", std::process::id()));
    std::fs::create_dir_all(&place).expect("a folder");
    std::fs::write(place.join("secret.txt"), "the secret").expect("a file");
    let address = format!(
        "file:///{}/secret.txt",
        place.display().to_string().replace(std::path::MAIN_SEPARATOR, "/").trim_start_matches('/')
    );
    let root = folder(
        "injection",
        &page(&format!(
            "// What a page does with markup an attacker got into it.
             const answer = await fetch('{address}');
             say('status=' + answer.status);
             say('body=' + (await answer.text()));
             const img = new Image(); img.src = '{address}'; document.body.appendChild(img);
             await sleep(500);
             say('image=' + img.naturalWidth);"
        )),
    );
    let run = probe(&root, &["--scenario", "counter", "--seconds", "6"], &[]);
    assert_eq!(run.messages, vec!["status=403", "body=forbidden", "image=0"], "{}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(place);
}

#[test]
fn a_pointer_event_says_where_it_is_in_the_element_and_how_big_the_element_is() {
    let root = folder(
        "events",
        &page(
            "const box = await wait('box');
             const inner = document.getElementById('inner');
             // clientX 160, clientY 70 is 60,20 inside a box at 100,50.
             inner.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, clientX: 160, clientY: 70, button: 2 }));
             await sleep(200);
             box.dispatchEvent(new WheelEvent('wheel', { bubbles: true, deltaY: -120, deltaX: 5 }));
             await sleep(200);
             box.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'k' }));
             await sleep(300);",
        ),
    );
    let run = probe(&root, &["--scenario", "events", "--seconds", "5"], &[]);
    let said = |kind: &str| {
        run.reports
            .iter()
            .find(|(key, _)| key == &format!("event_{kind}"))
            .map(|(_, value)| value.clone())
            .unwrap_or_else(|| panic!("no {kind} event: {} {:?}", run.stderr, run.reports))
    };
    let down = said("mousedown");
    assert!(down.contains("x=160 y=70 local=60,20 size=200x80 button=2"), "{down}");
    assert!(down.contains("current=box target=inner"), "{down}");
    assert!(said("wheel").contains("wheel=5,-120"), "{}", said("wheel"));
    assert!(said("keydown").contains("key=k"), "{}", said("keydown"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_form_submit_and_a_reload_do_not_throw_the_page_away() {
    // A submit navigates by default, and a navigation replaces the document
    // the host holds node ids for. A marker on `window` survives only if the
    // document does.
    let root = folder(
        "submit",
        &page(
            "window.marker = 'alive';
             const form = document.createElement('form');
             form.innerHTML = '<input name=q value=x><button id=go>go</button>';
             document.body.appendChild(form);
             let submitted = 0;
             form.addEventListener('submit', () => submitted++);
             document.getElementById('go').click();
             await sleep(1200);
             say('marker_after_submit=' + window.marker + ' submits=' + submitted + ' url=' + location.search);",
        ),
    );
    let run = probe(&root, &["--scenario", "counter", "--seconds", "5"], &[]);
    assert_eq!(run.messages, vec!["marker_after_submit=alive submits=1 url="], "{}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_field_written_before_what_it_depends_on_still_shows_its_value() {
    let root = folder(
        "fields",
        &page(
            "await wait('col');
             await sleep(300);
             const live = (id) => document.getElementById(id);
             say('select=' + live('sel').value);
             say('range=' + live('rng').value);
             say('number=' + live('num').value);
             say('checked=' + live('chk').checked);
             say('textarea=' + JSON.stringify(live('ta').value));
             say('color=' + live('col').value);
             // The user types '1.' into the number field: the field shows it, its value is empty.
             live('num').focus(); live('num').select();
             document.execCommand('insertText', false, '2.');
             await sleep(400);
             say('typed_number_shows=' + live('num').validity.badInput + '/' + live('num').value);
             // And chooses another option.
             live('sel').value = 'c';
             live('sel').dispatchEvent(new Event('change', { bubbles: true }));
             await sleep(400);
             say('select_after=' + live('sel').value);",
        ),
    );
    let run = probe(&root, &["--scenario", "fields", "--seconds", "6"], &[]);
    eprintln!("FIELDS: {:?}", run.messages);
    assert_eq!(
        run.messages[..6],
        ["select=b", "range=150", "number=1.5", "checked=true", "textarea=\"one\\ntwo\"", "color=#ff8800"],
        "{} {:?}",
        run.stderr,
        run.messages
    );
    assert_eq!(run.messages[7], "select_after=c", "{:?}", run.messages);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_path_in_text_rust_wrote_does_not_become_readable() {
    let place = std::env::temp_dir().join(format!("rustx-text-secret-{}", std::process::id()));
    std::fs::create_dir_all(&place).expect("a folder");
    let secret = place.join("secret.txt");
    std::fs::write(&secret, "the secret").expect("a file");
    let address = format!(
        "file:///{}",
        secret.display().to_string().replace(std::path::MAIN_SEPARATOR, "/").trim_start_matches('/')
    );
    let root = folder(
        "texts",
        &page(&format!(
            "await wait('typed');
             await sleep(300);
             say('title=' + document.querySelector('[title]').title.length);
             const answer = await fetch('{address}');
             say('status=' + answer.status);"
        )),
    );
    let run = {
        // The probe reads the path from its environment.
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_probe"));
        command
            .arg("--root")
            .arg(&root)
            .args(["--scenario", "texts", "--seconds", "5"])
            .env("PROBE_SECRET", &secret)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let output = command.output().expect("the probe runs");
        String::from_utf8_lossy(&output.stdout).to_string()
    };
    assert!(run.contains("REPORT message=status=403"), "{run}");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(place);
}
