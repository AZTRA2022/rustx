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

#[test]
fn a_write_posted_from_a_future_lands_on_a_frame() {
    // The whole round trip, in one process and with no window: a cell
    // on this thread, a future on the runtime's threads, a write that
    // crosses as a handle, and a frame that applies it.
    use rustx_dom::state::{InstanceId, Scope, Slot, cell, take_dirty};

    let id = InstanceId(8_001);
    let _scope = Scope::enter(id);
    let counter = cell(
        Slot {
            file: file!(),
            line: line!(),
            column: column!(),
        },
        || 0_i32,
    );
    let remote = counter.remote();
    let _ = take_dirty();

    let _task = task::spawn_future(async move {
        remote.update(|value| *value += 3);
    });

    // The future runs beside the frame, so the frame is what has to be
    // waited for — and nothing applies until one runs.
    assert!(
        within(PATIENCE, || {
            rustx_engine::frame::tick();
            counter.get() == 3
        }),
        "a write posted from a future never reached the cell"
    );
    assert!(
        take_dirty().contains(&id),
        "the write landed and nothing asked for a render"
    );
}

#[test]
fn dropping_the_token_refuses_what_the_future_had_already_posted() {
    // Abandoning a future stops it making new writes. What the final
    // review found is that it said nothing about the ones already in
    // the mailbox -- and a value found by a name outlives the page, so
    // they landed on the next one.
    use rustx_dom::state::{InstanceId, Scope, open_remote, remote_refused};

    open_remote();
    let id = InstanceId(8_010);
    let _scope = Scope::enter(id);
    let rows = rustx_dom::store::shared("engine-refused-rows", Vec::<u32>::new);
    let _ = rows.get();

    let posted = Arc::new(AtomicBool::new(false));
    let said = Arc::clone(&posted);
    let cancel = rustx_dom::state::Cancel::new();
    let remote = rows.remote_until(&cancel);
    let task = task::spawn_future_until(cancel, async move {
        remote.update(|held| held.push(1));
        said.store(true, Ordering::Release);
        // Long enough that the token is dropped while this is parked.
        tokio::time::sleep(Duration::from_secs(5)).await;
    });

    // The write is posted and waiting for a frame; the component now
    // leaves, which is what drops the token.
    assert!(
        within(PATIENCE, || posted.load(Ordering::Acquire)),
        "the future never posted its write"
    );
    let before = remote_refused();
    drop(task);

    rustx_engine::frame::tick();
    assert!(
        rows.get().is_empty(),
        "a cancelled task's write was applied: {:?}",
        rows.get()
    );
    assert_eq!(
        remote_refused(),
        before + 1,
        "the write was not counted as refused"
    );
}
