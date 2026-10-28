//! Smoke-test application for the engine.
//!
//! It opens a window on a local page whose buttons call Rust directly. Given
//! `--scenario=<name>`, the page drives itself and reports what it saw, which
//! is how the test suite watches the bridge work.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

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

    install_dom_natives();
    install_frame_natives();
    install_process_natives();
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
