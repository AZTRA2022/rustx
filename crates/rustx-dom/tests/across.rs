//! A value a future reads.
//!
//! State belongs to the render thread: a cell is an `Rc` around a
//! `RefCell`, and that is what makes a `let mut n = 0;` cost nothing.
//! A future therefore cannot read one — and the answer, until now, was
//! to refuse, which left the developer to build an `Arc<Mutex<…>>` by
//! hand and keep it up to date from a handler.
//!
//! What is proved here: a value the compiler saw a future read lives in
//! one place that both sides reach, with no copy published to anybody
//! and no clone added to a write.

use rustx_dom::state::{Cancel, open_remote};
use rustx_dom::store;

#[test]
fn a_future_reads_a_shared_value_the_render_thread_wrote() {
    open_remote();
    let cancel = Cancel::new();

    let clicks = store::shared("clicks_read_by_a_future", || 0_i32);
    let reach = clicks.reach_until(&cancel);

    // The write happens here, on the thread that draws, exactly as a
    // handler's does. Nothing is published anywhere.
    clicks.set(7);

    let seen = std::thread::spawn(move || reach.get())
        .join()
        .expect("the future's thread ran");
    assert_eq!(seen, 7, "a future did not see what the page wrote");
}

#[test]
fn a_future_sees_every_write_and_not_a_copy_taken_once() {
    open_remote();
    let cancel = Cancel::new();

    let count = store::shared("count_read_in_a_loop", || 0_u32);
    let reach = count.reach_until(&cancel);

    // A loop inside a future is the case that a copy taken at the top
    // of the block cannot serve: the second turn has to see the second
    // write.
    let mut seen = Vec::new();
    for value in 1..=3 {
        count.set(value);
        let reading = reach.clone();
        seen.push(
            std::thread::spawn(move || reading.get())
                .join()
                .expect("the future's thread ran"),
        );
    }
    assert_eq!(seen, vec![1, 2, 3], "a future saw a stale value");
}

#[test]
fn a_write_from_a_future_still_goes_through_the_frame() {
    // Reading is direct; writing is not, and that is deliberate. The
    // frame is what marks the page dirty, coalesces a thousand writes
    // into one render, and refuses a write whose component has gone.
    open_remote();
    let cancel = Cancel::new();

    let total = store::shared("total_written_by_a_future", || 0_i32);
    let reach = total.reach_until(&cancel);

    std::thread::spawn(move || reach.update(|held| *held += 5))
        .join()
        .expect("the future's thread ran");

    assert_eq!(total.get(), 0, "a write from a future skipped the frame");
    rustx_dom::state::apply_remote();
    assert_eq!(total.get(), 5, "the frame did not apply the write");
}
