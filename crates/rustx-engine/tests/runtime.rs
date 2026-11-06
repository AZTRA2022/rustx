//! A future beside the window.
//!
//! A component stays synchronous — it is asked for the screen as it is
//! now, so it cannot wait. What waits is a task, and this is its second
//! form: a `Future` instead of a closure, run by a tokio runtime that
//! lives on its own threads and never on the one that renders.
//!
//! No window is needed to prove any of it: a runtime starts, a future
//! runs, a token abandons it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use rustx_engine::{runtime, task};

/// Waits for `condition` rather than sleeping a guessed amount.
fn within(patience: Duration, condition: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + patience;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    condition()
}

const PATIENCE: Duration = Duration::from_secs(5);

#[test]
fn a_future_runs_and_says_so() {
    let ran = Arc::new(AtomicBool::new(false));
    let witness = Arc::clone(&ran);

    let _task = task::spawn_future(async move {
        witness.store(true, Ordering::Release);
    });

    assert!(
        within(PATIENCE, || ran.load(Ordering::Acquire)),
        "the future never ran"
    );
    assert!(
        runtime::started(),
        "a future ran and the runtime says it never started"
    );
}

#[test]
fn dropping_the_token_abandons_the_future_before_it_finishes() {
    let finished = Arc::new(AtomicBool::new(false));
    let started = Arc::new(AtomicBool::new(false));
    let witness = Arc::clone(&finished);
    let entered = Arc::clone(&started);

    let task = task::spawn_future(async move {
        entered.store(true, Ordering::Release);
        tokio::time::sleep(Duration::from_secs(5)).await;
        witness.store(true, Ordering::Release);
    });

    assert!(
        within(PATIENCE, || started.load(Ordering::Acquire)),
        "the future never got as far as its first await"
    );
    drop(task);

    // Long enough that an un-abandoned future would still be waiting,
    // and long enough that an abandoned one is certainly gone.
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !finished.load(Ordering::Acquire),
        "the token was dropped and the future finished anyway"
    );
}

#[test]
fn a_future_that_panics_leaves_the_runtime_alive() {
    let before = task::panics();
    let _doomed = task::spawn_future(async {
        panic!("a future panicked on purpose");
    });

    assert!(
        within(PATIENCE, || task::panics() > before),
        "a panicking future was never reported"
    );

    let after = Arc::new(AtomicU32::new(0));
    let witness = Arc::clone(&after);
    let _second = task::spawn_future(async move {
        witness.store(7, Ordering::Release);
    });

    assert!(
        within(PATIENCE, || after.load(Ordering::Acquire) == 7),
        "the runtime died with the future that panicked"
    );
}
