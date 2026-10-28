//! The counter of the design document, on screen.
//!
//! `app/page.rsx` holds this and nothing else:
//!
//! ```ignore
//! fn Counter() -> Element {
//!     let mut n = 0;
//!     return <button onClick={|| n += 1}>{n}</button>;
//! }
//! ```
//!
//! No `use_state`, no `signal`, no `move`, no `Rc<RefCell<…>>`, and no
//! conversion around `{n}`. If that block ever needs one more
//! annotation to work, the framework has failed at the thing it exists
//! for, and the fix belongs in the compiler rather than in this page.

// Components are PascalCase, as they are in TSX.
#![allow(non_snake_case)]
// And clippy reads the generated page, not the `.rsx`: its advice names
// lines in OUT_DIR that nobody wrote.
#![allow(clippy::needless_return, clippy::useless_vec)]

use std::path::Path;

use rustx_dom::state::{State, cell, with_key};
use rustx_dom::Element;
use rustx_engine::{
    App, AppConfig, Bootstrap, NativeError, Value, bootstrap, on_context_created, register_native,
    render,
};

include!(concat!(env!("OUT_DIR"), "/page.rs"));

fn main() {
    on_context_created(install);

    if let Bootstrap::Child { exit_code } = bootstrap() {
        std::process::exit(exit_code);
    }

    let index = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/index.html");
    let app = App::new(AppConfig {
        title: "Counter".into(),
        width: 640,
        height: 480,
        index,
    })
    .expect("the application's index page should be present");

    app.run().expect("the engine should start");
}

thread_local! {
    /// The order of the rows, so the page can ask for a rotation.
    static ORDER: std::cell::RefCell<Option<State<Vec<String>>>> =
        const { std::cell::RefCell::new(None) };
}

fn install() {
    // Two counters, side by side, from one `let mut n = 0;`.
    mount_when_ready("first", Counter);
    mount_when_ready("second", Counter);

    // A list of counters whose state is identified by key, so a
    // reorder carries each row's count with it.
    mount_when_ready("rows", || {
        let order = cell(
            rustx_dom::state::Slot {
                file: file!(),
                line: line!(),
                column: column!(),
            },
            || vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
        ORDER.with(|slot| *slot.borrow_mut() = Some(order.clone()));

        let mut list = Element::tag("ul");
        for key in order.get() {
            list = list.child(
                Element::tag("li")
                    .key(key.clone())
                    .child(with_key(&key, Counter))
                    .build(),
            );
        }
        list.build()
    });

    install_test_natives();
}

/// A V8 context exists before the body does, so mounting waits for the
/// first frame on which the host element is there.
fn mount_when_ready(id: &'static str, component: impl Fn() -> Element + 'static) {
    let component = std::rc::Rc::new(component);
    let done = std::cell::Cell::new(false);
    rustx_engine::on_frame(move || {
        if done.get() {
            return;
        }
        let component = std::rc::Rc::clone(&component);
        if render::mount_app(move || component(), id).is_ok() {
            done.set(true);
        }
    });
}

fn install_test_natives() {
    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("counter_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

    // Move the last row to the front. The reconciler moves the nodes;
    // the state has to come with them.
    register_native("counter_rotate", |_args| {
        ORDER.with(|slot| {
            if let Some(order) = slot.borrow().as_ref() {
                order.update(|rows| {
                    if let Some(last) = rows.pop() {
                        rows.insert(0, last);
                    }
                });
            }
        });
        Ok(Value::Null)
    });

    let report_path = argument("--report=");
    register_native("counter_report", move |args| {
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
            .map_err(|e| NativeError::new(format!("could not report: {e}")))?;
        Ok(Value::Null)
    });
}

fn argument(prefix: &str) -> Option<String> {
    std::env::args_os()
        .filter_map(|arg| arg.into_string().ok())
        .find_map(|arg| arg.strip_prefix(prefix).map(str::to_string))
}
