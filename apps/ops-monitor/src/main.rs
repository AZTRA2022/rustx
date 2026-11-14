//! Ops Monitor — the showcase's "Monitor: Remote Ressource App for
//! Agents", scoped down to what a demo app can honestly be.
//!
//! The marketing copy promises CPU/GPU/RAM/ROM/NETWORK/TOKENS tracking
//! with alerts on Email and WhatsApp. This app really reads CPU, RAM,
//! disk and per-process numbers from the machine it runs on — with the
//! `sysinfo` crate, not a mocked number — and really raises a system
//! notification when CPU crosses a threshold. It does not send an
//! email or a WhatsApp message: that is out of reach for a local demo,
//! so the alert is a real OS notification instead of a silent fake.
//! "Agents" here are the handful of heaviest processes on the machine,
//! which is the closest honest reading of "remote resource app for
//! agents" that a single-machine demo can show.

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Ops Monitor")
            .size(1320, 880)
            .description("Resource tracking for the agents running on this machine.")
            .publisher("rustx"),
    )?;

    app.run()
}
