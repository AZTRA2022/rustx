//! What the engine answers when there is no window at all.
//!
//! Every other integration test in this crate either starts a real CEF
//! window (`mod common`) or stays strictly inside the parts that never
//! touch V8. This file is the boundary between the two: it asks the
//! engine for things only a window can give and checks that the answer
//! is a refusal rather than a crash.
//!
//! It exists for two reasons. The first is the claim itself — a library
//! that aborts the process when it is used outside the context it
//! expects is a library nobody can test. The second is that the rest of
//! the headless suite is written against what this file establishes:
//! how far a test can drive the runtime on a machine with no window.

use rustx_engine::{Document, DomError, declare_api_version, render};

#[test]
fn asking_for_the_document_outside_a_window_answers_nothing() {
    // The version has to be declared before any call into libcef, in
    // every process. A test binary is a process.
    declare_api_version();
    assert!(
        Document::current().is_none(),
        "there is no page here, so there is no document"
    );
}

#[test]
fn mounting_at_an_id_outside_a_window_is_refused_and_says_why() {
    declare_api_version();
    let refused = rustx_engine::mount_at_id(rustx_dom::Element::Empty, "app");
    assert!(matches!(refused, Err(DomError::NoContext)), "{refused:?}");
}

#[test]
fn nothing_is_mounted_so_nothing_is_counted() {
    assert_eq!(render::roots(), 0);
    assert_eq!(
        render::unmount("app"),
        Ok(false),
        "there is no root called `app` to take off"
    );
}
