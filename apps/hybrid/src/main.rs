//! A page where Rust and JavaScript own one DOM between them.
//!
//! `app/page.rsx` writes the chart itself, with a component off npm:
//!
//! ```text
//! <js::recharts::LineChart data={data}>
//!     <js::recharts::XAxis dataKey="at" />
//! </js::recharts::LineChart>
//! ```
//!
//! The only JavaScript this application has is `web/src/main.jsx`, and
//! all it says is which libraries it uses. The bridge between the two is
//! the framework's.

use rustx::{App, Config, ToValue};

/// One point of the chart.
///
/// The chart is a library's; the data is the application's. `ToValue` is
/// what lets it cross, and the field names are what the library reads.
#[derive(ToValue, Clone)]
pub struct Point {
    pub at: f64,
    pub value: f64,
}

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("Hybrid").size(900, 640))?;

    app.run()
}
