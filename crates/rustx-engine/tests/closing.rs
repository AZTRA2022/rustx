//! Review Focus #5, as it actually happens.
//!
//! The proof application has a capture thread. It does not know the window
//! is closing, it is mid-loop when it happens, and it is still handing work
//! to the UI thread as the engine winds down. Calling `shutdown()` by hand
//! and seeing the next post refused proves the contract; it does not prove
//! the shutdown. This does: a thread posting as fast as it can, a window
//! that closes under it, and a process that has to be gone — not hung, not
//! crashed — when it is over.

mod common;

use std::sync::OnceLock;

use common::{Completion, run_scenario_to_completion};

fn run() -> &'static Completion {
    static RUN: OnceLock<Completion> = OnceLock::new();
    RUN.get_or_init(|| run_scenario_to_completion("closing"))
}

#[test]
fn the_window_closes_while_a_worker_thread_is_still_posting() {
    assert_eq!(
        run().report.get("closing"),
        "yes",
        "the scenario never reached the close:\n{}",
        run().report.raw()
    );
    let posted: u64 = run()
        .report
        .get("posted_before_closing")
        .parse()
        .expect("a number of applied updates");
    assert!(
        posted > 0,
        "the worker thread was not actually posting when the window closed:\n{}",
        run().report.raw()
    );
}

#[test]
fn the_process_exits_cleanly_with_a_worker_thread_still_running() {
    assert_eq!(
        run().exit,
        Some(0),
        "closing the window under a busy worker did not end the process cleanly \
         (None means it hung and had to be killed):\n{}",
        run().report.raw()
    );
}
