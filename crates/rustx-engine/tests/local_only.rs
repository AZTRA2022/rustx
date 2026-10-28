//! The precondition the unsandboxed render process rests on.
//!
//! The engine disables Chromium's sandbox because the only code in the
//! render process is the developer's own. That is only true if remote
//! content can never get in — and "can never" has to mean enforced, not
//! documented. A page that reaches a remote origin would be handed every
//! native function the application registered, unsandboxed, with the
//! user's full token.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("local_only"))
}

#[test]
fn navigating_to_a_remote_url_is_refused() {
    assert!(
        report().get("after_navigation").starts_with("file:///"),
        "the page left local content for {:?}",
        report().get("after_navigation")
    );
}

#[test]
fn a_remote_iframe_never_gets_a_context_with_native_functions() {
    assert_eq!(
        report().get("iframe_blocked"),
        "yes",
        "a remote iframe loaded:\n{}",
        report().raw()
    );
}

#[test]
fn the_engine_says_why_it_refused() {
    // A refusal the developer cannot see is a mystery, not a guard. The
    // count lives in whichever process did the refusing, so what is checked
    // here is the thing that actually reaches a human.
    let output = report().raw();

    assert!(
        output.contains("refused to load"),
        "the engine refused silently:\n{output}"
    );
    assert!(
        output.contains("example.com"),
        "the refusal did not name the URL:\n{output}"
    );
    assert!(
        output.contains("only loads local content"),
        "the refusal did not say why:\n{output}"
    );
}
