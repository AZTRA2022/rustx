//! What a sub-frame's context is allowed to take with it when it goes.
//!
//! A page is not one context. An iframe has its own, and so does every
//! frame a page creates and throws away — a preview pane, an embedded
//! local document, a widget being swapped out. Those come and go while the
//! application keeps running.
//!
//! So the release of a context must only end what belonged to it. If it
//! ended the application's frame queue, a page that closes a panel would
//! silently stop accepting work from every capture thread it has; if it
//! invalidated every DOM handle, Rust would start answering
//! `ContextDestroyed` for nodes of a document that is still on screen.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("subframe"))
}

#[test]
fn a_subframe_going_away_does_not_close_the_job_queue() {
    assert_eq!(
        report().get("post_after_subframe"),
        "Ok",
        "removing an iframe shut the application's queue down:\n{}",
        report().raw()
    );
}

#[test]
fn a_subframe_going_away_does_not_invalidate_the_main_frames_nodes() {
    assert_eq!(
        report().get("kept_node_after_subframe"),
        "usable",
        "a node of the main document went stale when an iframe was removed:\n{}",
        report().raw()
    );
}
