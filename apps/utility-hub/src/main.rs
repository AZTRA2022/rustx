//! Utility Hub — the showcase's "Management App for Tech & Fin
//! Startups".
//!
//! Convex, OpenRouter and an omniscient automation engine are not
//! what a demo needs to show the idea of one dashboard across a
//! startup's tech and finance sides. This app keeps three lists —
//! notifications, invoices, automations — and a roll-up over them;
//! "send by email/WhatsApp" is honestly a system notification, not a
//! real message leaving the machine.

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Utility Hub")
            .size(920, 680)
            .description("Tech and finance, one dashboard.")
            .publisher("rustx"),
    )?;

    app.run()
}
