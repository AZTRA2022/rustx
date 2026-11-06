//! A write that comes from somewhere else.
//!
//! State belongs to the render thread and to it alone: a cell is an `Rc`
//! around a `RefCell`, there is no lock on the hot path, and a
//! `let mut n = 0;` puts no thread bound on what a closure captures. A
//! future runs on the runtime's threads, so it cannot touch a cell — it
//! posts, and the frame applies.
//!
//! What is proved here: the write lands, it lands once a frame however
//! many of them there were, it is thrown away when the component that
//! would have read it is gone, and it is refused outright once the
//! window has closed.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use rustx_dom::state::{
    InstanceId, Scope, Slot, apply_remote, cell, close_remote, drop_instance, open_remote,
    remote_dropped, remote_refused, take_dirty,
};

/// A slot built where it is called, the way the generated code builds one.
macro_rules! here {
    () => {
        Slot {
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

#[test]
fn a_remote_write_lands_on_the_next_frame() {
    open_remote();
    let id = InstanceId(7_001);
    let slot = here!();
    let _scope = Scope::enter(id);
    let counter = cell(slot, || 0_i32);
    let remote = counter.remote();
    let _ = take_dirty();

    // Another thread entirely: `Remote` is `Send`, a cell is not.
    std::thread::spawn(move || remote.update(|value| *value += 5))
        .join()
        .expect("the writing thread");

    assert_eq!(
        counter.get(),
        0,
        "a remote write reached the cell without a frame"
    );
    apply_remote();
    assert_eq!(counter.get(), 5, "the write never landed");
    assert_eq!(
        take_dirty(),
        vec![id],
        "the write landed and nothing asked for a render"
    );
}

#[test]
fn a_remote_write_to_a_dropped_instance_is_thrown_away() {
    open_remote();
    let id = InstanceId(7_002);
    let slot = here!();
    let remote = {
        let _scope = Scope::enter(id);
        cell(slot, || 0_i32).remote()
    };

    remote.update(|value| *value += 1);
    // The component left before the frame that would have applied it.
    drop_instance(id);
    let before = remote_dropped();

    apply_remote();

    assert_eq!(
        remote_dropped(),
        before + 1,
        "a write to a component that had gone was not thrown away"
    );
}

#[test]
fn a_thousand_remote_writes_cost_one_render() {
    open_remote();
    let id = InstanceId(7_003);
    let slot = here!();
    let _scope = Scope::enter(id);
    let counter = cell(slot, || 0_u32);
    let remote = counter.remote();
    let _ = take_dirty();

    for _ in 0..1_000 {
        remote.update(|value| *value += 1);
    }
    apply_remote();

    assert_eq!(counter.get(), 1_000, "some writes were lost");
    assert_eq!(
        take_dirty().len(),
        1,
        "a thousand writes asked for more than one render"
    );
}

#[test]
fn a_remote_write_after_the_window_closed_is_refused() {
    open_remote();
    let id = InstanceId(7_004);
    let slot = here!();
    let _scope = Scope::enter(id);
    let counter = cell(slot, || 0_i32);
    let remote = counter.remote();

    // The window is going. Nothing will draw again, so a write has
    // nobody to reach.
    close_remote();
    let before = remote_refused();
    remote.update(|value| *value += 1);

    assert_eq!(
        remote_refused(),
        before + 1,
        "a write after the window closed was queued instead of refused"
    );
    open_remote();
    apply_remote();
    assert_eq!(
        counter.get(),
        0,
        "a refused write was applied on the next frame anyway"
    );
}

#[test]
fn a_remote_is_send_and_shared_between_threads() {
    open_remote();
    let id = InstanceId(7_005);
    let slot = here!();
    let _scope = Scope::enter(id);
    let counter = cell(slot, || 0_u32);
    let remote = counter.remote();
    let wrote = Arc::new(AtomicU32::new(0));

    let threads: Vec<std::thread::JoinHandle<()>> = (0..4)
        .map(|_| {
            let remote = remote.clone();
            let wrote = Arc::clone(&wrote);
            std::thread::spawn(move || {
                remote.update(|value| *value += 1);
                wrote.fetch_add(1, Ordering::Release);
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("a writing thread");
    }

    assert_eq!(wrote.load(Ordering::Acquire), 4);
    apply_remote();
    assert_eq!(counter.get(), 4, "a clone of a remote lost its cell");
}

#[test]
fn a_remote_write_to_a_shared_value_lands_on_the_next_frame() {
    // A `let mut` at the top of a page is a shared value with a name,
    // and a developer writes it exactly like a component's own — so a
    // future reaches it exactly like a component's own.
    open_remote();
    let id = InstanceId(7_006);
    let _scope = Scope::enter(id);
    let rows = rustx_dom::store::shared("remote-rows", Vec::<u32>::new);
    let _ = rows.get();
    let remote = rows.remote();
    let _ = take_dirty();

    std::thread::spawn(move || remote.update(|held| held.push(4)))
        .join()
        .expect("the writing thread");

    assert!(rows.get().is_empty(), "the write reached it without a frame");
    apply_remote();
    assert_eq!(rows.get(), vec![4], "the write never landed");
    assert_eq!(
        take_dirty(),
        vec![id],
        "the component that read it was not asked to render again"
    );
}
