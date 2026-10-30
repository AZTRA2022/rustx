//! A page where Rust and JavaScript own one DOM between them.
//!
//! Both rules of ownership are in `app/page.rsx`, which is twelve lines:
//!
//! - `<js::Chart points={…} />` is a node JavaScript owns. rustx puts
//!   the container down, says what goes in it and with what props, and
//!   never looks inside.
//! - `#panel-host` is a node JavaScript builds, and the panel it builds
//!   asks rustx for a component of its own — which becomes a root like
//!   any other, re-rendered by the same frame.
//!
//! What runs the JavaScript half is `assets/components.js`: thirty hand
//! written lines standing in for React and react-dom, which are an npm
//! dependency and belong to the packaging plan. What is proved here is
//! the contract those two rules make — which is what React consumes.

// Components are PascalCase, as they are in TSX.
#![allow(non_snake_case)]
// And clippy reads the generated page, not the `.rsx`.
#![allow(clippy::needless_return, clippy::useless_vec)]

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use rustx_dom::Element;
use rustx_dom::store::store;
use rustx_engine::{
    App, AppConfig, Bootstrap, NativeError, Value, bootstrap, foreign, on_context_created,
    register_native, router,
};

/// How many points the chart is showing. A store, so the page and the
/// component React mounted see the same one.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Points(pub u32);

/// How many times the page has been drawn, so the scenario can see that
/// one frame drew both sides.
static DRAWS: AtomicU32 = AtomicU32::new(0);

/// Add a point: both halves of the page are watching this one value.
pub fn add_point() {
    let points = store::<Points>();
    let now = points.get().0;
    points.set(Points(now + 1));
}

/// Leave the page, which takes the JavaScript component's container with
/// it — and JavaScript has to be told first.
pub fn leave() {
    router::navigate("/gone".to_string());
}

include!(concat!(env!("OUT_DIR"), "/routes.rs"));

fn main() {
    on_context_created(install);

    if let Bootstrap::Child { exit_code } = bootstrap() {
        std::process::exit(exit_code);
    }

    let index = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/index.html");
    let app = App::new(AppConfig {
        title: "Hybrid".into(),
        width: 900,
        height: 640,
        index,
    })
    .expect("the application's index page should be present");

    app.run().expect("the engine should start");
}

fn install() {
    // What JavaScript is allowed to mount. A counter, because what the
    // scenario has to see is that it is a live component and not a
    // snapshot: it reads the same store the chart's props come from.
    foreign::register_component("Counter", counter);

    let started = std::cell::Cell::new(false);
    rustx_engine::on_frame(move || {
        if started.get() {
            return;
        }
        if router::start(route_table(), "app", "/", gone).is_ok() {
            started.set(true);
        }
    });

    install_test_natives();
}

/// The component JavaScript's panel mounts. It shows the same number
/// the chart does, which is how one frame is seen to have drawn both.
fn counter() -> Element {
    DRAWS.fetch_add(1, Ordering::Release);
    Element::tag("span")
        .attribute(
            "id",
            rustx_dom::AttributeValue::Text("counter-in-panel".into()),
        )
        .child(Element::Text(store::<Points>().get().0.to_string()))
        .build()
}

/// Where `leave` goes. An ordinary component, and the page it replaces
/// takes the JavaScript container with it.
fn gone() -> Element {
    Element::tag("p")
        .attribute("id", rustx_dom::AttributeValue::Text("gone".into()))
        .child(Element::Text("gone".into()))
        .build()
}

fn install_test_natives() {
    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("hybrid_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

    register_native("hybrid_points", |_args| {
        Ok(Value::String(store::<Points>().get().0.to_string()))
    });

    let report_path = argument("--report=");
    register_native("hybrid_report", move |args| {
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
        .filter_map(|argument| argument.into_string().ok())
        .find_map(|argument| {
            argument
                .strip_prefix(prefix)
                .map(|value| value.to_string())
        })
}
