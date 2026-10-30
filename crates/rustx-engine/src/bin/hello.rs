//! Smoke-test application for the engine.
//!
//! It opens a window on a local page whose buttons call Rust directly. Given
//! `--scenario=<name>`, the page drives itself and reports what it saw, which
//! is how the test suite watches the bridge work.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use rustx_dom::{AttributeValue, Element, Handler};
use rustx_engine::{
    Document, DomError, Node,
    App, AppConfig, Bootstrap, NativeError, Value, bootstrap, on_context_created, on_frame,
    poster, process,
    register_native,
};

fn main() {
    // Registered before bootstrap: the render process is this same binary and
    // never runs past that line, so this is its only chance to be told what
    // the page may call.
    on_context_created(install_natives);

    if let Bootstrap::Child { exit_code } = bootstrap() {
        std::process::exit(exit_code);
    }

    let index = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/hello/index.html");

    let app = App::new(AppConfig {
        title: "rustx - hello".into(),
        width: 960,
        height: 640,
        index,
    })
    .expect("the bundled example page should be present");

    report_line("browser_pid", &std::process::id().to_string());
    println!("WINDOW_OPENED args={:?}", std::env::args().collect::<Vec<_>>());
    app.run().expect("the engine should start");
}

fn install_natives() {
    // Plain Rust state, owned by the render process, with no cell of
    // framework machinery around it.
    let counter = Cell::new(0_i64);
    register_native("rustx_count", move |_args| {
        counter.set(counter.get() + 1);
        Ok(Value::Number(counter.get() as f64))
    });

    register_native("rustx_refusals", |_args| {
        Ok(Value::String(rustx_engine::refusals().to_string()))
    });

    register_native("rustx_panic", |_args| -> Result<Value, NativeError> {
        panic!("this native function panics on purpose");
    });

    register_native("rustx_needs_one", |args| {
        if args.len() != 1 {
            return Err(NativeError::new(format!(
                "expected 1 argument, got {}",
                args.len()
            )));
        }
        Ok(Value::Null)
    });

    register_native("rustx_echo", |args| {
        let parts: Vec<String> = args
            .iter()
            .map(|value| match value {
                Value::Null => "null".to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => format!("{n}"),
                Value::String(s) => s.clone(),
            })
            .collect();
        Ok(Value::String(parts.join("/")))
    });

    // A CEF sub-process does not reliably inherit the parent's stdout, so
    // the scenario reports through a file the test harness names.
    let report_path = argument("--report=");
    register_native("rustx_report", move |args| {
        let [Value::String(key), Value::String(value)] = args else {
            return Err(NativeError::new("expected 2 string arguments"));
        };
        let Some(path) = report_path.as_deref() else {
            return Err(NativeError::new("no --report=<path> was given"));
        };
        let line = format!("REPORT {key}={value}\n");
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()))
            .map_err(|e| NativeError::new(format!("could not write the report: {e}")))?;
        Ok(Value::Null)
    });

    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("rustx_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

    install_mount_natives();
    install_render_natives();
    install_lifecycle_natives();
    install_dom_natives();
    install_frame_natives();
    install_task_natives();
    install_process_natives();
}

// What the background tasks of the `task` scenario report about
// themselves. They are statics rather than thread-locals because a task
// body runs on its own thread: it writes here, the UI thread reads.
static ENDLESS_STOPPED: AtomicBool = AtomicBool::new(false);
static POST_AFTER_CANCEL: Mutex<Option<&'static str>> = Mutex::new(None);
static KEPT_TASK_STOPPED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// The token of the endless task, held where the scenario can drop it.
    static ENDLESS: RefCell<Option<rustx_engine::task::Task>> = const { RefCell::new(None) };
    /// The token of the task that panics, kept so that the panic is the
    /// only reason the body ever ends.
    static PANICKER: RefCell<Option<rustx_engine::task::Task>> = const { RefCell::new(None) };
    static TASK_TICKS: Cell<u64> = const { Cell::new(0) };
}

/// A capture thread's whole life, as the proof-of-concept software will
/// write it: it is started by a component, it posts what it produces, and
/// it stops because its token went away.
fn install_task_natives() {
    register_native("rustx_task_start", |_args| {
        let task = rustx_engine::task::spawn(|alive| {
            while alive.get() {
                std::thread::sleep(std::time::Duration::from_millis(5));
                let _ = alive.post(|| TASK_TICKS.with(|ticks| ticks.set(ticks.get() + 1)));
            }
            // Past the loop, the token is gone. What the task has left to
            // say must be refused rather than queued.
            let refused = match alive.post(|| {}) {
                Ok(()) => "accepted",
                Err(_) => "refused",
            };
            *POST_AFTER_CANCEL
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(refused);
            ENDLESS_STOPPED.store(true, Ordering::Release);
        });
        ENDLESS.with(|slot| *slot.borrow_mut() = Some(task));
        Ok(Value::Null)
    });

    register_native("rustx_task_running", |_args| {
        Ok(Value::String(
            ENDLESS
                .with(|slot| slot.borrow().as_ref().map(rustx_engine::task::Task::is_running))
                .map(|running| running.to_string())
                .unwrap_or_else(|| "none".to_string()),
        ))
    });

    register_native("rustx_task_ticks", |_args| {
        Ok(Value::String(TASK_TICKS.with(Cell::get).to_string()))
    });

    // Dropping the token is the whole cancellation API.
    register_native("rustx_task_cancel", |_args| {
        ENDLESS.with(|slot| *slot.borrow_mut() = None);
        Ok(Value::Null)
    });

    register_native("rustx_task_stopped", |_args| {
        Ok(Value::String(
            if ENDLESS_STOPPED.load(Ordering::Acquire) {
                "yes".to_string()
            } else {
                "no".to_string()
            },
        ))
    });

    register_native("rustx_task_post_after_cancel", |_args| {
        Ok(Value::String(
            POST_AFTER_CANCEL
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .unwrap_or("pending")
                .to_string(),
        ))
    });

    register_native("rustx_task_panics", |_args| {
        let task = rustx_engine::task::spawn(|_alive| {
            panic!("a background task panicked on purpose");
        });
        PANICKER.with(|slot| *slot.borrow_mut() = Some(task));
        Ok(Value::Null)
    });

    // The sentence the engine said about it, read from inside the render
    // process because a CEF sub-process's stderr does not reliably reach
    // the harness.
    register_native("rustx_task_panic_notice", |_args| {
        Ok(Value::String(
            rustx_engine::task::last_panic().unwrap_or_else(|| "nothing was said".to_string()),
        ))
    });

    // A component that starts a thread, holding the token the way the
    // compiler's `task(…)` will: as a value kept by the component.
    register_native("rustx_mount_ticker", |_args| {
        rustx_engine::render::mount_app(
            || {
                rustx_dom::state::keep_at(
                    rustx_dom::state::Slot {
                        file: file!(),
                        line: line!(),
                        column: column!(),
                    },
                    || {
                        rustx_engine::task::spawn(|alive| {
                            while alive.get() {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                            }
                            KEPT_TASK_STOPPED.store(true, Ordering::Release);
                        })
                    },
                );
                Element::tag("p")
                    .attribute("id", AttributeValue::Text("ticker".into()))
                    .child(Element::Text("ticking".into()))
                    .build()
            },
            "ticker",
        )?;
        Ok(Value::Null)
    });

    register_native("rustx_unmount_ticker", |_args| {
        Ok(Value::String(
            rustx_engine::render::unmount("ticker")?.to_string(),
        ))
    });

    register_native("rustx_task_after_unmount", |_args| {
        Ok(Value::String(
            if KEPT_TASK_STOPPED.load(Ordering::Acquire) {
                "stopped".to_string()
            } else {
                "running".to_string()
            },
        ))
    });
}

/// Trees built in Rust and put on screen, the way the compiler's output
/// will put them there.
fn install_mount_natives() {
    // `<ul><li>a</li><li><b>c</b></li></ul>` — two levels, text at depth,
    // siblings in the order they were written.
    register_native("rustx_mount_tree", |_args| {
        let tree = Element::tag("ul")
            .child(Element::tag("li").child(Element::Text("a".into())).build())
            .child(
                Element::tag("li")
                    .child(Element::tag("b").child(Element::Text("c".into())).build())
                    .build(),
            )
            .build();
        mount_into(tree, "tree")
    });

    register_native("rustx_mount_empty", |_args| {
        mount_into(Element::Empty, "empty")
    });

    register_native("rustx_mount_fragment", |_args| {
        let tree = Element::Fragment(vec![
            Element::tag("i").child(Element::Text("a".into())).build(),
            Element::tag("i").child(Element::Text("b".into())).build(),
        ]);
        mount_into(tree, "fragment")
    });

    // The whole claim of the framework, in one button: the closure below
    // is ordinary Rust, it captures an `Rc` with no thread bound, and a
    // click runs it on the click's own stack.
    let clicks = Rc::new(Cell::new(0_u32));
    let counted = Rc::clone(&clicks);
    register_native("rustx_mount_button", move |_args| {
        let counter = Rc::clone(&counted);
        let tree = Element::tag("button")
            .attribute("id", AttributeValue::Text("mounted".into()))
            .handler(
                "onClick",
                Handler::new(move |_event| counter.set(counter.get() + 1)),
            )
            .child(Element::Text("go".into()))
            .build();
        mount_into(tree, "button")
    });

    register_native("rustx_clicks", move |_args| {
        Ok(Value::String(clicks.get().to_string()))
    });
}

// A counter and a sibling that never changes, mounted the way a `.rsx`
// page is mounted — the component function is re-run, and only what
// changed reaches the DOM.
thread_local! {
    static SIBLING_RENDERS: Cell<u64> = const { Cell::new(0) };
    /// A clone of the counter's handle, so a worker thread can mutate
    /// the very state the component reads.
    static COUNTER_STATE: RefCell<Option<rustx_dom::state::State<u64>>> = const { RefCell::new(None) };
}

fn install_render_natives() {
    register_native("rustx_mount_counter", |_args| {
        rustx_engine::render::mount_app(
            || {
                let n = rustx_dom::state::cell(
                    rustx_dom::state::Slot {
                        file: file!(),
                        line: line!(),
                        column: column!(),
                    },
                    || 0_u64,
                );
                let clicked = n.clone();
                COUNTER_STATE.with(|slot| *slot.borrow_mut() = Some(n.clone()));
                Element::tag("div")
                    .child(
                        Element::tag("button")
                            .attribute("id", AttributeValue::Text("tick".into()))
                            .handler(
                                "onClick",
                                Handler::new(move |_| clicked.update(|v| *v += 1)),
                            )
                            .child(Element::Text(n.get().to_string()))
                            .build(),
                    )
                    .child(
                        Element::tag("span")
                            .attribute("id", AttributeValue::Text("fixed".into()))
                            .child(Element::Text("fixed".into()))
                            .build(),
                    )
                    .build()
            },
            "counter",
        )?;
        Ok(Value::Null)
    });

    // A second root nobody touches. Its render count must not move.
    register_native("rustx_mount_sibling", |_args| {
        rustx_engine::render::mount_app(
            || {
                SIBLING_RENDERS.with(|count| count.set(count.get() + 1));
                Element::tag("p").child(Element::Text("still".into())).build()
            },
            "sibling",
        )?;
        Ok(Value::Null)
    });

    register_native("rustx_counter_state", |_args| {
        Ok(Value::String(counter_state_value()))
    });
    register_native("rustx_sibling_renders", |_args| {
        Ok(Value::String(SIBLING_RENDERS.with(Cell::get).to_string()))
    });
    register_native("rustx_renders", |_args| {
        Ok(Value::String(rustx_engine::render::renders().to_string()))
    });

    // A thousand mutations from a worker thread, as fast as it can.
    register_native("rustx_flood", |_args| {
        let poster = poster();
        std::thread::spawn(move || {
            for _ in 0..1000 {
                if poster.post(bump_the_counter).is_err() {
                    return;
                }
            }
        });
        Ok(Value::Null)
    });
}

/// Runs on the UI thread, which is where the state lives. The worker
/// thread never touches it — it posts this.
fn bump_the_counter() {
    COUNTER_STATE.with(|slot| {
        if let Some(state) = slot.borrow().as_ref() {
            state.update(|value| *value += 1);
        }
    });
}

fn mount_into(tree: Element, id: &str) -> Result<Value, NativeError> {
    rustx_engine::mount_at_id(tree, id)?;
    Ok(Value::Null)
}

/// Everything the page can ask Rust to do to the DOM.
fn install_dom_natives() {
    // How many times a context has run the scenario, so the page knows
    // whether it is before or after the reload.
    let visits = Cell::new(0_i64);
    register_native("rustx_visit", move |_args| {
        visits.set(visits.get() + 1);
        Ok(Value::Number(visits.get() as f64))
    });

    // A list built entirely from Rust: create the element, give it text and
    // a class, attach it.
    let items = Cell::new(0_i64);
    register_native("rustx_add_item", move |_args| {
        let document = Document::current().ok_or(DomError::NoContext)?;
        items.set(items.get() + 1);

        let item = document.create_element("li")?;
        item.set_text(&format!("item {}", items.get()))?;
        item.set_attribute("class", "from-rust")?;

        let list = document
            .get_element_by_id("list")?
            .ok_or_else(|| DomError::Failed("no #list in the page".into()))?;
        list.append_child(&item)?;

        Ok(Value::Null)
    });

    register_native("rustx_count_items", |_args| {
        let document = Document::current().ok_or(DomError::NoContext)?;
        Ok(Value::Number(document.count_matching("#list li")? as f64))
    });

    register_native("rustx_last_item_class", |_args| {
        let document = Document::current().ok_or(DomError::NoContext)?;
        let last = document
            .query_selector("#list li:last-child")?
            .ok_or_else(|| DomError::Failed("the list is empty".into()))?;
        match last.attribute("class")? {
            Some(class) => Ok(Value::String(class)),
            None => Ok(Value::Null),
        }
    });

    register_native("rustx_remove_last_item", |_args| {
        let document = Document::current().ok_or(DomError::NoContext)?;
        let last = document
            .query_selector("#list li:last-child")?
            .ok_or_else(|| DomError::Failed("the list is empty".into()))?;
        last.remove()?;
        Ok(Value::Null)
    });

    // A node deliberately kept past the life of its context, to prove that
    // using it afterwards is an error and not a crash.
    let kept: Rc<RefCell<Option<Node>>> = Rc::new(RefCell::new(None));

    let keeper = Rc::clone(&kept);
    register_native("rustx_keep_node", move |_args| {
        let document = Document::current().ok_or(DomError::NoContext)?;
        let node = document
            .get_element_by_id("list")?
            .ok_or_else(|| DomError::Failed("no #list in the page".into()))?;
        *keeper.borrow_mut() = Some(node);
        Ok(Value::Null)
    });

    register_native("rustx_use_kept_node", move |_args| {
        let node = kept
            .borrow()
            .clone()
            .ok_or_else(|| DomError::Failed("no node was kept".into()))?;
        node.set_text("this should never be written")?;
        Ok(Value::Null)
    });
}

/// Natives that let the page see which process its Rust runs in.
fn install_process_natives() {
    register_native("rustx_pid", |_args| {
        Ok(Value::Number(std::process::id() as f64))
    });

    register_native("rustx_role", |_args| {
        Ok(Value::String(format!("{:?}", process::role())))
    });

    // Does nothing on purpose: what it measures is the cost of the call
    // itself, which is the whole claim.
    register_native("rustx_nop", |_args| Ok(Value::Null));

    // Reads the live document, so the page can hand Rust a value and ask
    // for it back without anything in between.
    register_native("rustx_read_out", |_args| {
        let document = Document::current().ok_or(DomError::NoContext)?;
        let node = document
            .get_element_by_id("out")?
            .ok_or_else(|| DomError::Failed("no #out in the page".into()))?;
        Ok(Value::String(node.text()?))
    });

    // Reads the DOM and writes it back, from Rust — the round trip the
    // design document says must stay in nanoseconds.
    register_native("rustx_touch_dom", |_args| {
        let document = Document::current().ok_or(DomError::NoContext)?;
        let node = document
            .get_element_by_id("out")?
            .ok_or_else(|| DomError::Failed("no #out in the page".into()))?;
        let text = node.text()?;
        node.set_text(&text)?;
        Ok(Value::Null)
    });
}

// Application state lives on the UI thread and nowhere else. A posted job
// reaches it by running there — never by carrying a reference across.
thread_local! {
    static COUNTER: Cell<u64> = const { Cell::new(0) };
    static DOM_WRITES: Cell<u64> = const { Cell::new(0) };
    static SHOWN: Cell<u64> = const { Cell::new(u64::MAX) };
    static JOB_THREAD: RefCell<String> = const { RefCell::new(String::new()) };
    static WORKER_THREAD: RefCell<String> = const { RefCell::new(String::new()) };
    static FRAMES_AT_CHECK: Cell<u64> = const { Cell::new(0) };
    // A frame callback registered before the panicking one. If a panic
    // dropped the registered callbacks, this one would stop climbing.
    static SENTINEL: Cell<u64> = const { Cell::new(0) };
    static SENTINEL_AT_CHECK: Cell<u64> = const { Cell::new(0) };
}

/// The frame loop, a worker thread, and the coalescing between them.
fn install_frame_natives() {
    // Rendering happens here and nowhere else: once per frame, showing
    // whatever the state became in between.
    on_frame(|| {
        let counter = COUNTER.with(Cell::get);
        if SHOWN.with(Cell::get) == counter {
            return;
        }
        let Some(document) = Document::current() else {
            return;
        };
        let Ok(Some(node)) = document.get_element_by_id("out") else {
            return;
        };
        if node.set_text(&counter.to_string()).is_ok() {
            SHOWN.with(|shown| shown.set(counter));
            DOM_WRITES.with(|writes| writes.set(writes.get() + 1));
        }
    });

    on_frame(|| SENTINEL.with(|count| count.set(count.get() + 1)));

    register_native("rustx_start_thread", |_args| {
        let poster = poster();
        std::thread::spawn(move || {
            // Only `Send` data crosses: a string, and a function with no
            // captures that reaches UI-thread state by running there.
            let worker = thread_id();
            let _ = poster.post(move || {
                WORKER_THREAD.with(|thread| *thread.borrow_mut() = worker);
            });

            for _ in 0..10_000 {
                if poster.post(bump_counter).is_err() {
                    return;
                }
            }
        });
        Ok(Value::Null)
    });

    // A job that panics, and a frame callback that panics once. Both are
    // the shape a hardware-driven application hits sooner or later.
    register_native("rustx_panic_in_job", |_args| {
        poster()
            .post(|| panic!("a posted job panicked on purpose"))
            .map_err(|error| NativeError::new(error.to_string()))?;
        Ok(Value::Null)
    });

    register_native("rustx_panic_in_frame_callback", |_args| {
        let fired = Cell::new(false);
        on_frame(move || {
            if !fired.get() {
                fired.set(true);
                panic!("a frame callback panicked on purpose");
            }
        });
        Ok(Value::Null)
    });

    // Proof the loop is still turning: the frame counter has to keep
    // climbing after the panics.
    register_native("rustx_frames_still_climbing", |_args| {
        FRAMES_AT_CHECK.with(|at| at.set(rustx_engine::frame::frame_count()));
        SENTINEL_AT_CHECK.with(|at| at.set(SENTINEL.with(Cell::get)));
        Ok(Value::Null)
    });

    // Did the loop itself survive the panics?
    register_native("rustx_alive_since_check", |_args| {
        let before = FRAMES_AT_CHECK.with(Cell::get);
        Ok(Value::String(alive_or_frozen(
            before,
            rustx_engine::frame::frame_count(),
        )))
    });

    // And did the callbacks standing next to the panicking one survive?
    register_native("rustx_callbacks_alive_since_check", |_args| {
        let before = SENTINEL_AT_CHECK.with(Cell::get);
        Ok(Value::String(alive_or_frozen(
            before,
            SENTINEL.with(Cell::get),
        )))
    });

    register_native("rustx_ui_thread", |_args| Ok(Value::String(thread_id())));
    register_native("rustx_frames", |_args| {
        Ok(Value::String(rustx_engine::frame::frame_count().to_string()))
    });
    register_native("rustx_counter", |_args| {
        Ok(Value::String(COUNTER.with(Cell::get).to_string()))
    });
    register_native("rustx_dom_writes", |_args| {
        Ok(Value::String(DOM_WRITES.with(Cell::get).to_string()))
    });
    register_native("rustx_job_thread", |_args| {
        Ok(Value::String(JOB_THREAD.with(|t| t.borrow().clone())))
    });
    register_native("rustx_worker_thread", |_args| {
        Ok(Value::String(WORKER_THREAD.with(|t| t.borrow().clone())))
    });

    // A capture thread's shape: it never stops on its own, and it is still
    // producing when the user closes the window.
    register_native("rustx_start_endless_thread", |_args| {
        let poster = poster();
        std::thread::spawn(move || {
            while poster.post(bump_counter).is_ok() {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });
        Ok(Value::Null)
    });

    // Is the queue still open? Unlike the one below, this asks without
    // closing anything.
    register_native("rustx_try_post", |_args| {
        Ok(Value::String(match poster().post(|| {}) {
            Ok(()) => "Ok".to_string(),
            Err(error) => error.to_string(),
        }))
    });

    // What a closing window does to the queue, and what a thread still
    // producing work is told in return.
    register_native("rustx_post_after_close", |_args| {
        rustx_engine::frame::shutdown();
        Ok(Value::String(match poster().post(|| {}) {
            Ok(()) => "Ok".to_string(),
            Err(error) => error.to_string(),
        }))
    });
}

fn alive_or_frozen(before: u64, now: u64) -> String {
    if now > before {
        "alive".to_string()
    } else {
        format!("frozen at {now}")
    }
}

/// Runs on the UI thread, because that is where the queue is drained.
fn bump_counter() {
    COUNTER.with(|counter| counter.set(counter.get() + 1));
    JOB_THREAD.with(|thread| {
        let mut thread = thread.borrow_mut();
        if thread.is_empty() {
            *thread = thread_id();
        }
    });
}

fn thread_id() -> String {
    format!("{:?}", std::thread::current().id())
}

/// Appends a line to the report file, from whichever process calls it.
fn report_line(key: &str, value: &str) {
    let Some(path) = argument("--report=") else {
        return;
    };
    let line = format!("REPORT {key}={value}\n");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
}

/// The value of a `--name=value` argument this process was launched with.
fn argument(prefix: &str) -> Option<String> {
    std::env::args().find_map(|arg| arg.strip_prefix(prefix).map(str::to_string))
}

/// A value whose only job is to say when it was dropped — the shape of
/// a capture session or a file handle a component holds.
struct Releases(&'static std::thread::LocalKey<Cell<u64>>);

impl Drop for Releases {
    fn drop(&mut self) {
        self.0.with(|count| count.set(count.get() + 1));
    }
}

thread_local! {
    static DROPS: Cell<u64> = const { Cell::new(0) };
    /// What the rows of the keyed list released.
    static ROW_DROPS: Cell<u64> = const { Cell::new(0) };
    /// The rows themselves, so the page can take one away.
    static ROWS: RefCell<Option<rustx_dom::state::State<Vec<String>>>> =
        const { RefCell::new(None) };
    /// Which device name the props component is showing.
    static SELECTED: RefCell<Option<rustx_dom::state::State<usize>>> =
        const { RefCell::new(None) };
}

const DEVICES: [&str; 3] = ["Webcam", "Micro", "Écran"];

/// A plain function taking a parameter. Not a component, not state —
/// just a value flowing in, which is all a prop is.
fn device_row(label: &str) -> Element {
    Element::tag("li")
        .attribute("id", AttributeValue::Text("row".into()))
        .child(Element::Text(label.to_string()))
        .build()
}

fn install_lifecycle_natives() {
    register_native("rustx_mount_holder", |_args| {
        rustx_engine::render::mount_app(
            || {
                let held = rustx_dom::state::cell(
                    rustx_dom::state::Slot {
                        file: file!(),
                        line: line!(),
                        column: column!(),
                    },
                    || Releases(&DROPS),
                );
                let ticks = rustx_dom::state::cell(
                    rustx_dom::state::Slot {
                        file: file!(),
                        line: line!(),
                        column: column!(),
                    },
                    || 0_u64,
                );
                // `held` is not Clone, so it is read without copying.
                held.with(|_| ());
                let bumped = ticks.clone();
                Element::tag("button")
                    .attribute("id", AttributeValue::Text("holder".into()))
                    .handler("onClick", Handler::new(move |_| bumped.update(|v| *v += 1)))
                    .child(Element::Text(ticks.get().to_string()))
                    .build()
            },
            "holder",
        )?;
        Ok(Value::Null)
    });

    register_native("rustx_unmount_holder", |_args| {
        Ok(Value::String(
            rustx_engine::render::unmount("holder")?.to_string(),
        ))
    });
    register_native("rustx_drops", |_args| {
        Ok(Value::String(DROPS.with(Cell::get).to_string()))
    });
    register_native("rustx_live_instances", |_args| {
        Ok(Value::String(
            rustx_dom::state::live_instances().to_string(),
        ))
    });

    // A keyed list whose rows hold something with a `Drop`. A row that
    // leaves has to release it, and that cleanup is only reachable from
    // the end of a render — which is why this is driven through the
    // engine rather than by hand.
    register_native("rustx_mount_rows", |_args| {
        rustx_engine::render::mount_app(
            || {
                let rows = rustx_dom::state::cell(
                    rustx_dom::state::Slot {
                        file: file!(),
                        line: line!(),
                        column: column!(),
                    },
                    || vec!["a".to_string(), "b".to_string()],
                );
                ROWS.with(|slot| *slot.borrow_mut() = Some(rows.clone()));

                let mut list = Element::tag("ul");
                for key in rows.get() {
                    let row = rustx_dom::state::with_key(&key, || {
                        let held = rustx_dom::state::cell(
                            rustx_dom::state::Slot {
                                file: file!(),
                                line: line!(),
                                column: column!(),
                            },
                            || Releases(&ROW_DROPS),
                        );
                        held.with(|_| ());
                        Element::Text(key.clone())
                    });
                    list = list.child(Element::tag("li").key(key.clone()).child(row).build());
                }
                list.build()
            },
            "rows",
        )?;
        Ok(Value::Null)
    });

    register_native("rustx_remove_row", |_args| {
        ROWS.with(|slot| {
            if let Some(rows) = slot.borrow().as_ref() {
                rows.update(|rows| {
                    rows.pop();
                });
            }
        });
        Ok(Value::Null)
    });

    register_native("rustx_row_drops", |_args| {
        Ok(Value::String(ROW_DROPS.with(Cell::get).to_string()))
    });

    register_native("rustx_mount_props", |_args| {
        rustx_engine::render::mount_app(
            || {
                let selected = rustx_dom::state::cell(
                    rustx_dom::state::Slot {
                        file: file!(),
                        line: line!(),
                        column: column!(),
                    },
                    || 0_usize,
                );
                SELECTED.with(|slot| *slot.borrow_mut() = Some(selected.clone()));
                Element::tag("ul")
                    .child(device_row(DEVICES[selected.get().min(2)]))
                    .build()
            },
            "props",
        )?;
        Ok(Value::Null)
    });

    register_native("rustx_select", |args| {
        let [Value::Number(index)] = args else {
            return Err(NativeError::new("expected one number"));
        };
        let index = *index as usize;
        SELECTED.with(|slot| {
            if let Some(state) = slot.borrow().as_ref() {
                state.set(index);
            }
        });
        Ok(Value::Null)
    });
}

/// The counter's value, read from the state rather than from the page.
fn counter_state_value() -> String {
    COUNTER_STATE.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|state| state.get().to_string())
            .unwrap_or_default()
    })
}
