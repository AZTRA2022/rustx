//! A context that follows a closed one takes writes again.
//!
//! A file of its own, for the same reason its sibling has one: this
//! shuts the frame queue down and stops the runtime, and a test that
//! does that cannot share a process with one that asks whether the
//! runtime is running.

use rustx_engine::frame;

#[test]
fn a_new_context_takes_remote_writes_again() {
    // `frame::open()` reopens the queue for a fresh context. It has to
    // reopen the mailbox with it, or a future spawned for the new page
    // runs and every write it makes is refused in silence.
    use rustx_dom::state::{InstanceId, Scope, Slot, apply_remote, cell};

    frame::shutdown();
    frame::open();

    let id = InstanceId(9_100);
    let _scope = Scope::enter(id);
    let counter = cell(
        Slot {
            file: file!(),
            line: line!(),
            column: column!(),
        },
        || 0_i32,
    );
    counter.remote().update(|value| *value += 4);
    apply_remote();

    assert_eq!(
        counter.get(),
        4,
        "the mailbox stayed closed for the new context"
    );
}
