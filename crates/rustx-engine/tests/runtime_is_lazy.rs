//! An application with no `async` does not pay for a runtime.
//!
//! A file of its own, and one test in it, because the claim is about a
//! whole process: cargo gives one process per integration test file, and
//! any other test that spawned a future first would have started the
//! runtime before this one looked.

use std::time::Duration;

use rustx_engine::{runtime, task};

#[test]
fn nothing_starts_a_runtime_until_a_future_asks_for_one() {
    assert!(
        !runtime::started(),
        "a runtime was running before anything asked for one"
    );

    // A task that is a thread, which is what every application wrote
    // before this plan: still no runtime.
    let thread = task::spawn(|alive| {
        while alive.get() {
            std::thread::sleep(Duration::from_millis(1));
        }
    });
    assert!(
        !runtime::started(),
        "a task that is a thread started a tokio runtime"
    );
    drop(thread);

    let _future = task::spawn_future(async {});
    assert!(
        runtime::started(),
        "a future was spawned and no runtime started"
    );
}
