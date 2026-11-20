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
                .handler("onClick", Handler::new(move |_| clicked.update(|n| *n += 1)))
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
    if let Some(name) = scenario {
        let component: fn() -> Element = match name.as_str() {
            "counter" => counter,
            "echo" => echo,
            "keyed" => keyed,
            other => {
                eprintln!("probe: no scenario `{other}`");
                std::process::exit(2);
            }
        };
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
