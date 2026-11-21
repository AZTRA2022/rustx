//! A window for the tests to drive from outside.
//!
//! Serves a folder, prints what the host sees as `REPORT key=value` lines
//! on stdout, and closes itself after `--seconds`. A test cannot run tao's
//! loop in its own thread, since on Windows and macOS it has to be the main
//! thread, so the loop lives in this process and the test reads its report.
//!
//! `--root DIR` (required), `--seconds N` (default 6), `--allow ORIGIN`
//! (repeatable), `--data-dir DIR`, `--scenario NAME`. External links are not
//! opened: they are reported as `opened=<url>`.
//!
//! A scenario mounts a component of the application under `#app`, once the
//! page says it is ready: `counter`, `echo` (a controlled field) or `keyed`
//! (a keyed list that rotates).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::time::Duration;

use rustx_dom::state::{Slot, State, cell};
use rustx_dom::{AttributeValue, Element, Handler};
use rustx_webview::host::{Config, Hooks, run};

fn report(key: &str, value: &str) {
    println!("REPORT {key}={}", value.replace(['\n', '\r'], " "));
}

fn attr(name: &str, value: &str) -> (String, AttributeValue) {
    (name.to_string(), AttributeValue::Text(value.to_string()))
}

macro_rules! slot {
    () => {
        Slot {
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

fn counter() -> Element {
    let clicks: State<u64> = cell(slot!(), || 0);
    let clicked = clicks.clone();
    Element::tag("div")
        .child(
            Element::tag("button")
                .attribute("id", attr("id", "inc").1)
                .handler(
                    "onClick",
                    Handler::new(move |_| clicked.update(|n| *n += 1)),
                )
                .child(Element::Text(clicks.get().to_string()))
                .build(),
        )
        .child(
            Element::tag("span")
                .attribute("id", attr("id", "fixed").1)
                .child(Element::Text("fixed".to_string()))
                .build(),
        )
        .build()
}

fn echo() -> Element {
    let text: State<String> = cell(slot!(), String::new);
    let typed = text.clone();
    // A slow handler, so the page has typed more by the time the host
    // answers: what makes a stale value possible.
    let delay: u64 = std::env::var("PROBE_INPUT_DELAY_MS")
        .ok()
        .and_then(|ms| ms.parse().ok())
        .unwrap_or(0);
    Element::tag("div")
        .child(
            Element::tag("input")
                .attribute("id", attr("id", "field").1)
                .attribute("value", attr("value", &text.get()).1)
                .handler(
                    "onInput",
                    Handler::new(move |event| {
                        std::thread::sleep(Duration::from_millis(delay));
                        typed.set(event.value)
                    }),
                )
                .build(),
        )
        .child(
            Element::tag("span")
                .attribute("id", attr("id", "echo").1)
                .child(Element::Text(text.get()))
                .build(),
        )
        .build()
}

/// A box whose handlers say what the host was told about each event, so a
/// test can check the whole payload with no application in the way.
fn events() -> Element {
    let said = |kind: &'static str| {
        Handler::new(move |event| {
            report(
                &format!("event_{kind}"),
                &format!(
                    "x={} y={} local={},{} size={}x{} button={} wheel={},{} current={} target={} key={} value={}",
                    event.x,
                    event.y,
                    event.local_x,
                    event.local_y,
                    event.width,
                    event.height,
                    event.button,
                    event.delta_x,
                    event.delta_y,
                    event.current_id,
                    event.id,
                    event.key,
                    event.value
                ),
            )
        })
    };
    Element::tag("div")
        .attribute("id", attr("id", "box").1)
        .attribute(
            "style",
            attr(
                "style",
                "position:absolute;left:100px;top:50px;width:200px;height:80px",
            )
            .1,
        )
        .handler("onMouseDown", said("mousedown"))
        .handler("onWheel", said("wheel"))
        .handler("onContextMenu", said("contextmenu"))
        .handler("onKeyDown", said("keydown"))
        .child(
            Element::tag("span")
                .attribute("id", attr("id", "inner").1)
                .child(Element::Text("inner".to_string()))
                .build(),
        )
        .build()
}

/// Every kind of field, controlled from Rust, with `value` written before the
/// attributes and children it depends on, as a developer writes it.
fn fields() -> Element {
    let select: State<String> = cell(slot!(), || "b".to_string());
    let (chose, shown) = (select.clone(), select.get());
    let number: State<String> = cell(slot!(), || "1.5".to_string());
    let typed = number.clone();
    let range: State<String> = cell(slot!(), || "150".to_string());
    let dragged = range.clone();
    let option = |name: &str| {
        Element::tag("option")
            .attribute("value", attr("value", name).1)
            .child(Element::Text(name.to_string()))
            .build()
    };
    let on = |state: State<String>, kind: &'static str| {
        Handler::new(move |event| {
            report(&format!("changed_{kind}"), &event.value);
            state.set(event.value)
        })
    };
    Element::tag("div")
        .child(
            Element::tag("select")
                .attribute("id", attr("id", "sel").1)
                .attribute("value", attr("value", &shown).1)
                .handler("onChange", on(chose, "sel"))
                .child(option("a"))
                .child(option("b"))
                .child(option("c"))
                .build(),
        )
        .child(
            Element::tag("input")
                .attribute("id", attr("id", "rng").1)
                .attribute("value", attr("value", &range.get()).1)
                .attribute("type", attr("type", "range").1)
                .attribute("min", attr("min", "0").1)
                .attribute("max", attr("max", "200").1)
                .handler("onInput", on(dragged, "rng"))
                .build(),
        )
        .child(
            Element::tag("input")
                .attribute("id", attr("id", "num").1)
                .attribute("type", attr("type", "number").1)
                .attribute("value", attr("value", &number.get()).1)
                .handler("onInput", on(typed, "num"))
                .build(),
        )
        .child(
            Element::tag("input")
                .attribute("id", attr("id", "chk").1)
                .attribute("type", attr("type", "checkbox").1)
                .attribute("checked", AttributeValue::Flag(true))
                .build(),
        )
        .child(
            Element::tag("textarea")
                .attribute("id", attr("id", "ta").1)
                .attribute("value", attr("value", "one\r\ntwo").1)
                .build(),
        )
        .child(
            Element::tag("input")
                .attribute("id", attr("id", "col").1)
                .attribute("type", attr("type", "color").1)
                .attribute("value", attr("value", "#ff8800").1)
                .build(),
        )
        .build()
}

/// Text a person or a file wrote, put in places that are not addresses: a
/// path in it must not become readable by the page.
fn texts() -> Element {
    let path = std::env::var("PROBE_SECRET").unwrap_or_default();
    let address = format!(
        "file:///{}",
        path.replace('\\', "/").trim_start_matches('/')
    );
    Element::tag("div")
        .attribute("title", attr("title", &address).1)
        .attribute("data-note", attr("data-note", &address).1)
        .child(
            Element::tag("input")
                .attribute("id", attr("id", "typed").1)
                .attribute("value", attr("value", &address).1)
                .build(),
        )
        .child(Element::Text(address))
        .build()
}

/// A component that owns something with a `Drop`, as one that holds a file or
/// a device does: closing the window must run it.
struct Owned;

impl Drop for Owned {
    fn drop(&mut self) {
        if let Ok(path) = std::env::var("PROBE_DROP_FILE") {
            let _ = std::fs::write(path, "dropped");
        }
    }
}

fn owner() -> Element {
    let _held: State<Option<std::rc::Rc<Owned>>> = cell(slot!(), || Some(std::rc::Rc::new(Owned)));
    Element::tag("div")
        .attribute("id", attr("id", "owner").1)
        .build()
}

/// The old engine's parity checks that need no page: a thousand rows with one
/// changing, ten thousand posted updates, and a component that is released.
/// Everything is reported as `engine_*`, then `engine_done`.
fn engine() {
    use rustx_webview::render::{last_render, mount_app, renders, unmount};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU64, Ordering};

    thread_local! {
        static ROWS: RefCell<Option<State<Vec<u32>>>> = const { RefCell::new(None) };
        static COUNT: RefCell<Option<State<u64>>> = const { RefCell::new(None) };
    }
    static DROPS: AtomicU64 = AtomicU64::new(0);
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::SeqCst);
        }
    }

    let heavy = || {
        let rows: State<Vec<u32>> = cell(slot!(), || (0..1000).collect());
        ROWS.with(|held| *held.borrow_mut() = Some(rows.clone()));
        let mut list = Element::tag("ul").attribute("id", attr("id", "rows").1);
        for (at, value) in rows.get().into_iter().enumerate() {
            list = list.child(
                Element::tag("li")
                    .key(at.to_string())
                    .child(Element::Text(format!("row {value}")))
                    .build(),
            );
        }
        list.build()
    };
    let counter = || {
        let count: State<u64> = cell(slot!(), || 0);
        COUNT.with(|held| *held.borrow_mut() = Some(count.clone()));
        Element::tag("span")
            .attribute("id", attr("id", "n").1)
            .child(Element::Text(count.get().to_string()))
            .build()
    };
    let owned = || {
        let _guard: State<Option<Rc<Guard>>> = cell(slot!(), || Some(Rc::new(Guard)));
        Element::tag("i").build()
    };

    let step = std::cell::Cell::new(0u32);
    rustx_webview::on_frame(move || {
        if rustx_webview::Document::current().is_none() {
            return;
        }
        match step.get() {
            0 => {
                let started = std::time::Instant::now();
                mount_app(heavy, "app").expect("mounts");
                report(
                    "engine_first_render_ms",
                    &started.elapsed().as_millis().to_string(),
                );
                step.set(1);
            }
            1 => {
                // One row of a thousand changes.
                ROWS.with(|held| {
                    held.borrow()
                        .as_ref()
                        .expect("mounted")
                        .update(|rows| rows[500] = 9999)
                });
                step.set(2);
            }
            2 => {
                let (micros, patches) = last_render();
                report("engine_one_row_patches", &patches.to_string());
                report("engine_one_row_micros", &micros.to_string());
                step.set(3);
            }
            3 => {
                let _ = unmount("app");
                mount_app(counter, "app").expect("mounts");
                let before = renders();
                report("engine_renders_before_posts", &before.to_string());
                // Ten thousand increments from four threads, handed to the loop.
                let poster = rustx_webview::poster();
                let done = Arc::new(AtomicU64::new(0));
                for _ in 0..4 {
                    let (poster, done) = (poster.clone(), Arc::clone(&done));
                    std::thread::spawn(move || {
                        for _ in 0..2500 {
                            let _ = poster.post(|| {
                                COUNT.with(|held| {
                                    held.borrow().as_ref().expect("mounted").update(|n| *n += 1)
                                })
                            });
                        }
                        done.fetch_add(1, Ordering::SeqCst);
                    });
                }
                DONE.with(|held| *held.borrow_mut() = Some(done));
                step.set(4);
            }
            4 => {
                let finished = DONE.with(|held| {
                    held.borrow()
                        .as_ref()
                        .is_some_and(|d| d.load(Ordering::SeqCst) == 4)
                });
                let count = COUNT.with(|held| held.borrow().as_ref().map(|c| c.get()).unwrap_or(0));
                if finished && count == 10_000 {
                    report("engine_posted_total", &count.to_string());
                    report("engine_renders_for_posts", &renders().to_string());
                    step.set(5);
                }
            }
            5 => {
                let _ = unmount("app");
                mount_app(owned, "app").expect("mounts");
                step.set(6);
            }
            6 => {
                let _ = unmount("app");
                report(
                    "engine_drops_after_unmount",
                    &DROPS.load(Ordering::SeqCst).to_string(),
                );
                report("engine_done", "1");
                step.set(7);
            }
            _ => {}
        }
    });
}

thread_local! {
    static DONE: std::cell::RefCell<Option<std::sync::Arc<std::sync::atomic::AtomicU64>>> = const { std::cell::RefCell::new(None) };
}
use std::sync::Arc;

fn drawing() -> Element {
    Element::tag("svg")
        .attribute("id", attr("id", "drawing").1)
        .attribute("width", attr("width", "100").1)
        .attribute("height", attr("height", "50").1)
        .child(
            Element::tag("rect")
                .attribute("id", attr("id", "box").1)
                .attribute("x", attr("x", "10").1)
                .attribute("y", attr("y", "10").1)
                .attribute("width", attr("width", "30").1)
                .attribute("height", attr("height", "20").1)
                .build(),
        )
        .build()
}

fn keyed() -> Element {
    let order: State<Vec<u32>> = cell(slot!(), || vec![1, 2, 3, 4]);
    let rotated = order.clone();
    let mut list = Element::tag("ul")
        .attribute("id", attr("id", "list").1)
        // Text before the keyed children, the way `<ul>Items:<li key=…>` is written.
        .child(Element::Text("Items:".to_string()));
    for n in order.get() {
        list = list.child(
            Element::tag("li")
                .key(format!("k{n}").as_str())
                .attribute("id", attr("id", &format!("row-{n}")).1)
                .child(Element::Text(n.to_string()))
                .build(),
        );
    }
    Element::tag("div")
        .child(
            Element::tag("button")
                .attribute("id", attr("id", "rotate").1)
                .handler(
                    "onClick",
                    Handler::new(move |_| {
                        rotated.update(|rows| rows.rotate_left(1));
                    }),
                )
                .child(Element::Text("rotate".to_string()))
                .build(),
        )
        .child(list.build())
        .build()
}

fn main() {
    let mut root = None;
    let mut seconds = 6u64;
    let mut allow = Vec::new();
    let mut data_dir = None;
    let mut scenario = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut value = || arguments.next().unwrap_or_default();
        match argument.as_str() {
            "--root" => root = Some(PathBuf::from(value())),
            "--seconds" => seconds = value().parse().unwrap_or(seconds),
            "--allow" => allow.push(value()),
            "--data-dir" => data_dir = Some(PathBuf::from(value())),
            "--scenario" => scenario = Some(value()),
            "--files" => rustx_webview::files::allow(value()),
            other => {
                eprintln!("probe: unknown argument `{other}`");
                std::process::exit(2);
            }
        }
    }
    let Some(root) = root else {
        eprintln!("probe: --root is required");
        std::process::exit(2);
    };

    let mut config = Config::new("probe", root);
    config.allow = allow;
    if data_dir.is_some() {
        config.data_dir = data_dir;
    }
    config.devtools = false;

    // The thread that owns the window is the one that runs this, so the
    // frame callback and the loop share their state.
    if scenario.as_deref() == Some("controls") {
        use rustx_webview::{NativeError, Value, register_native};
        register_native("order", |args| match args {
            [Value::String(word)] => match rustx_os::window::Order::from_word(word) {
                Some(order) => {
                    rustx_webview::window::command(order);
                    Ok(Value::Null)
                }
                None => Err(NativeError::new("no such order")),
            },
            _ => Err(NativeError::new("order takes a word")),
        });
        config.scripts = vec![rustx_webview::natives_script()];
    } else if scenario.as_deref() == Some("natives") {
        use rustx_webview::{NativeError, Value, register_native};
        register_native("add", |args| match args {
            [Value::Number(a), Value::Number(b)] => Ok(Value::Number(a + b)),
            _ => Err(NativeError::new("add takes two numbers")),
        });
        register_native("fails", |_| Err(NativeError::new("it failed")));
        register_native("panics", |_| panic!("on purpose"));
        rustx_webview::register_async_native("slow", |args| {
            let again = args.first().cloned().unwrap_or(Value::Null);
            async move {
                tokio::time::sleep(Duration::from_millis(1500)).await;
                Ok(again)
            }
        });
        rustx_webview::register_async_native("slow_fails", |_| async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Err(NativeError::new("slow failure"))
        });
        rustx_webview::register_async_native("slow_panics", |_| async {
            if true {
                panic!("on purpose");
            }
            Ok(Value::Null)
        });
        config.scripts = vec![rustx_webview::natives_script()];
    } else if scenario.as_deref() == Some("engine") {
        engine();
    } else if let Some(name) = scenario {
        let component: fn() -> Element = match name.as_str() {
            "counter" => counter,
            "echo" => echo,
            "keyed" => keyed,
            "events" => events,
            "fields" => fields,
            "texts" => texts,
            "owner" => owner,
            "drawing" => drawing,
            other => {
                eprintln!("probe: no scenario `{other}`");
                std::process::exit(2);
            }
        };
        if name == "owner" {
            rustx_webview::register_native("order", |args| {
                if let [rustx_webview::Value::String(word)] = args
                    && let Some(order) = rustx_os::window::Order::from_word(word)
                {
                    rustx_webview::window::command(order);
                }
                Ok(rustx_webview::Value::Null)
            });
            config.scripts = vec![rustx_webview::natives_script()];
        }
        let mounted = std::cell::Cell::new(false);
        rustx_webview::on_frame(move || {
            if mounted.get() || rustx_webview::Document::current().is_none() {
                return;
            }
            match rustx_webview::render::mount_app(component, "app") {
                Ok(()) => mounted.set(true),
                Err(error) => eprintln!("probe: could not mount: {error}"),
            }
        });
    }

    let hooks = Hooks {
        // The loop ends when a timer says so, through a proxy that may
        // cross threads.
        on_start: Box::new(move |control| {
            let control = control.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(seconds));
                report("done", "1");
                control.exit();
            });
        }),
        on_message: Box::new(|message, _| report("message", message)),
        open_external: Box::new(|url| report("opened", url)),
        ..Hooks::default()
    };

    // `run` returns only an error: `Ok` holds an `Infallible`.
    let Err(error) = run(config, hooks);
    eprintln!("probe: {error}");
    std::process::exit(1);
}
