//! A value a component waits for without waiting.
//!
//! A component is asked for the screen as it is now, so it cannot stop
//! to wait. What it can do is ask once, draw nothing, and be drawn
//! again when the answer arrives — which is what `let x = f().await;`
//! in a page turns into. No window is needed to prove it: a cell, a
//! frame that drains the mailbox, and a future.

use std::time::{Duration, Instant};

use rustx_dom::state::{InstanceId, Scope, Slot, apply_remote, cell, open_remote};
use rustx_engine::task;

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
fn the_first_render_gets_nothing_and_a_later_one_gets_the_value() {
    open_remote();
    let _scope = Scope::enter(InstanceId(9_101));

    // The two places the compiler writes: where the `let` is, and where
    // the `.await` was. The same two on every render, which is what
    // makes the future be asked for once.
    let value = here!();
    let spawn = here!();
    let render = || task::awaited(cell(value, || None::<u32>), spawn, || async { 41u32 + 1 });

    assert_eq!(render(), None, "the render waited for the future");

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut got = None;
    while Instant::now() < deadline {
        // What a frame does: drain what the futures posted, then draw.
        apply_remote();
        got = render();
        if got.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(got, Some(42), "the value never reached the cell");
    assert_eq!(render(), Some(42), "it did not stay there");
}
