//! The runtime goes when the window does.
//!
//! A file of its own, because the test shuts the frame queue down and a
//! queue that is down refuses everything afterwards: no other test can
//! share the process with it.

use rustx_engine::{frame, runtime, task};

#[test]
fn the_runtime_goes_when_the_frame_queue_does() {
    let _future = task::spawn_future(async {});
    assert!(
        runtime::started(),
        "a future was spawned and no runtime started"
    );

    // What the engine calls when a context goes away, in whichever
    // process. A future left running after its window has gone has
    // nobody to post to and nothing to draw.
    frame::shutdown();

    assert!(
        !runtime::started(),
        "the window went and the async runtime stayed"
    );
}
