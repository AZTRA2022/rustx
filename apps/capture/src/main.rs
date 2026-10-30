//! Two screens, a layout that outlives both, and a stream that closes
//! itself.
//!
//! The whole application is `app/`:
//!
//! ```text
//! app/layout.rsx            the bar above every screen
//! app/page.rsx              the device list,            at /
//! app/device/[id]/page.rsx  one device's preview,       at /device/[id]
//! ```
//!
//! Nothing below mounts a screen, and nothing unmounts one. `main`
//! hands the generated table to the router and that is the end of it.
//! Opening a device is `navigate`; going back is `back`. The preview's
//! stream is a kept value, so leaving the screen closes it, and the
//! bar's thread belongs to the layout, so it keeps running while the
//! screens under it change.

// Components are PascalCase, as they are in TSX.
#![allow(non_snake_case)]
// And clippy reads the generated pages, not the `.rsx`: its advice names
// lines in OUT_DIR that nobody wrote.
#![allow(clippy::needless_return, clippy::useless_vec)]

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use rustx_dom::Element;
use rustx_dom::store::store;
use rustx_engine::{
    App, AppConfig, Bootstrap, NativeError, Value, bootstrap, on_context_created, register_native,
    router,
};

/// How long the application has been up, in ticks of the layout's
/// thread. A store, so the bar and anything else see the same one.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ticks(pub u32);

/// Which device the user picked. Read by both screens, handed down by
/// neither.
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct Selected(pub String);

/// How many streams have been closed, so a test can see that leaving a
/// preview closed the one it had opened.
static CLOSED: AtomicU32 = AtomicU32::new(0);

/// A capture stream, standing in for the hardware the OS layer will
/// bring. What matters is that it is opened by a component and closed by
/// nobody.
pub struct Stream {
    pub device: String,
}

impl Stream {
    pub fn open(device: &str) -> Self {
        Self {
            device: device.to_string(),
        }
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        CLOSED.fetch_add(1, Ordering::Release);
    }
}

/// Picking a device from the list: remember it, and go to its screen.
pub fn open(device: &str) {
    store::<Selected>().set(Selected(device.to_string()));
    router::navigate(format!("/device/{device}"));
}

include!(concat!(env!("OUT_DIR"), "/routes.rs"));

fn main() {
    on_context_created(install);

    if let Bootstrap::Child { exit_code } = bootstrap() {
        std::process::exit(exit_code);
    }

    let index = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/index.html");
    let app = App::new(AppConfig {
        title: "Console".into(),
        width: 800,
        height: 600,
        index,
    })
    .expect("the application's index page should be present");

    app.run().expect("the engine should start");
}

fn install() {
    // A V8 context exists before the body does, so the router waits for
    // the frame on which its host element is there.
    let started = std::cell::Cell::new(false);
    rustx_engine::on_frame(move || {
        if started.get() {
            return;
        }
        if router::start(route_table(), "app", "/", not_found).is_ok() {
            started.set(true);
        }
    });

    install_test_natives();
}

/// A path nobody declared. Every application needs one, and it is an
/// ordinary component.
fn not_found() -> Element {
    Element::tag("p")
        .attribute("id", rustx_dom::AttributeValue::Text("preview".into()))
        .child(Element::Text("no such device".into()))
        .build()
}

fn install_test_natives() {
    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("capture_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

    register_native("capture_ticks", |_args| {
        Ok(Value::String(store::<Ticks>().get().0.to_string()))
    });

    register_native("capture_closed_streams", |_args| {
        Ok(Value::String(CLOSED.load(Ordering::Acquire).to_string()))
    });

    let report_path = argument("--report=");
    register_native("capture_report", move |args| {
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
