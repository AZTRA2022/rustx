//! Rust building and mutating the DOM, and letting go of it safely.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

/// The scenario visits the page twice: it builds nodes from Rust, keeps one
/// across a reload, then tries to use it once its context is gone.
fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("dom"))
}

#[test]
fn rust_creates_and_mutates_nodes() {
    assert_eq!(
        report().get("last_item"),
        "item 1",
        "Rust did not build the list item:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("item_count"),
        "2",
        "the second item did not land:\n{}",
        report().raw()
    );
}

#[test]
fn rust_sets_attributes_and_removes_nodes() {
    assert_eq!(
        report().get("item_class"),
        "from-rust",
        "the attribute Rust set did not stick:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("count_after_remove"),
        "1",
        "removing a node from Rust did not take:\n{}",
        report().raw()
    );
}

#[test]
fn a_node_outliving_its_context_returns_an_error() {
    // A handle kept across a navigation must not be a use-after-free, must
    // not panic, and must not quietly do nothing. It must say so.
    assert!(
        report().get("stale_error").contains("context destroyed"),
        "a stale node did not report its context was gone: {:?}",
        report().get("stale_error")
    );
}
