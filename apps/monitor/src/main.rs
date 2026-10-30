//! A monitor fed by a background thread.
//!
//! `app/page.rsx` holds this and nothing else:
//!
//! ```ignore
//! fn Panel() -> Element {
//!     let ticks = store::<Ticks>();
//!
//!     task(move |alive| {
//!         while alive.get() {
//!             std::thread::sleep(std::time::Duration::from_millis(50));
//!             let _ = alive.post(|| store::<Ticks>().update(|ticks| ticks.0 += 1));
//!         }
//!     });
//!
//!     return <p id="panel">{ticks.get().0}</p>;
//! }
//! ```
//!
//! That is the shape of the proof-of-concept software's capture panel,
//! without the hardware: a thread that never stops on its own, a value
//! two distant components read, and no cleanup written anywhere. There
//! is no `use_effect`, no dependency array, and no `move` written by
//! hand around the state. The task is a value the component keeps, so it
//! is dropped when the component leaves, so the thread stops.

// Components are PascalCase, as they are in TSX.
#![allow(non_snake_case)]
// And clippy reads the generated page, not the `.rsx`: its advice names
// lines in OUT_DIR that nobody wrote.
#![allow(clippy::needless_return, clippy::useless_vec)]

use std::path::Path;

use rustx_dom::Element;
use rustx_dom::store::store;
use rustx_engine::{
    App, AppConfig, Bootstrap, NativeError, Value, bootstrap, on_context_created, register_native,
    render,
};

/// What the thread produces. A store is found by its type, so the type
/// is the name of the application's one tick count.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ticks(pub u32);

include!(concat!(env!("OUT_DIR"), "/page.rs"));

fn main() {
    on_context_created(install);

    if let Bootstrap::Child { exit_code } = bootstrap() {
        std::process::exit(exit_code);
    }

    let index = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/index.html");
    let app = App::new(AppConfig {
        title: "Monitor".into(),
        width: 640,
        height: 480,
        index,
    })
    .expect("the application's index page should be present");

    app.run().expect("the engine should start");
}

fn install() {
    // Two components that are not each other's parent, each reading the
    // same store. Nothing is handed down.
    mount_when_ready("header-host", Header);
    mount_when_ready("panel-host", Panel);

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
    register_native("monitor_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

    // Taking the panel off the screen is all the scenario does to stop
    // the thread. There is no `stop` to call.
    register_native("monitor_unmount_panel", |_args| {
        Ok(Value::String(render::unmount("panel-host")?.to_string()))
    });

    // The count read from the store rather than from the page, so that
    // "is it still ticking?" can be asked after the panel is gone.
    register_native("monitor_ticks", |_args| {
        Ok(Value::String(store::<Ticks>().get().0.to_string()))
    });

    let report_path = argument("--report=");
    register_native("monitor_report", move |args| {
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
