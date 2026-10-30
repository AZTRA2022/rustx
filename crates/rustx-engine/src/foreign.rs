//! The container a React component lives in.
//!
//! rustx puts an empty element on the page with the id the compiler gave
//! the boundary, and that is the whole of rustx's side: what goes inside
//! is React's, and the handover itself is the next task of this plan.
//! Until that exists, a boundary that is rendered says so once — a
//! container that silently stays empty is the kind of nothing that costs
//! an afternoon.

use std::cell::Cell;

/// What an application sees if it renders a boundary before the bridge
/// to React is installed.
pub const NOTICE: &str =
    "rustx: a node React owns was rendered, and no React bridge is installed — \
     the container is on the page and empty";

thread_local! {
    static SAID: Cell<bool> = const { Cell::new(false) };
}

/// Say it once per window, not once per frame.
pub(crate) fn report_missing_bridge() {
    if !SAID.with(Cell::get) {
        SAID.with(|said| said.set(true));
        eprintln!("{NOTICE}");
    }
}
