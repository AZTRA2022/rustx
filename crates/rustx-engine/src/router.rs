//! One screen at a time, chosen by a path.
//!
//! The table comes from the compiler (`routes.rs`, generated from the
//! directory the pages live in) and the matcher from `rustx-dom`. What
//! is left is the part that has to be right: going from one screen to
//! another must keep what the screens that *stay* are holding, and let
//! go of what the screen that left was holding.
//!
//! Nothing here unmounts anything. The router renders one component —
//! the layouts from the outside in, the page at the centre — through
//! `component_at`, so each is a component written at a place. A layout
//! that still encloses the new page is reached at the same place, so it
//! keeps its cells; the page that was left is not reached at all, so the
//! end of the render releases it, and the tasks it held stop because
//! their tokens were among its cells. That is plan 4's rule, applied to
//! a whole subtree.
//!
//! The current route is a store, which is what makes `navigate` work
//! from a worker thread: it is a mutation like any other, it marks the
//! screen for the next frame, and the frame draws once however many
//! times it was called.

use std::cell::RefCell;

use rustx_dom::Element;
use rustx_dom::route::{Params, Route, match_route};
use rustx_dom::state::{Slot, component_at};
use rustx_dom::store::store;

use crate::dom::DomError;

/// The path the application is on. A store, so that a component reading
/// it is re-rendered when it changes.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Current(String);

impl Default for Current {
    fn default() -> Self {
        Self("/".to_string())
    }
}

thread_local! {
    /// The table, the screen for a path nobody declared, and where the
    /// application has been.
    static ROUTER: RefCell<Option<Mounted>> = const { RefCell::new(None) };
    /// Where the application asked to go before there was a router to
    /// take it there.
    ///
    /// A script runs when the JavaScript context is created, and the
    /// router mounts on the first frame where its host element exists —
    /// which is later. A navigation asked for in between used to be
    /// overwritten by the starting path: the screen never came up and
    /// nothing said why. It is held instead, and honoured on start.
    static ASKED_FOR: RefCell<Option<String>> = const { RefCell::new(None) };
}

struct Mounted {
    table: &'static [Route],
    not_found: fn() -> Element,
    /// Paths visited, oldest first. The window's own history is the OS
    /// layer's business; this is the application's.
    history: Vec<String>,
}

/// Put the application's screens under `host`, starting at `path`.
///
/// One mount for the whole application: the router is the component, and
/// which screen it draws is a value it reads.
pub fn start(
    table: &'static [Route],
    host: &str,
    path: &str,
    not_found: fn() -> Element,
) -> Result<(), DomError> {
    // Where the application has been is the starting path, and then
    // wherever it asked to go while it was waiting for us.
    let mut history = vec![path.to_string()];
    if let Some(asked_for) = ASKED_FOR.with(|held| held.borrow_mut().take()) {
        history.push(asked_for);
    }
    store::<Current>().set(Current(
        history.last().cloned().unwrap_or_else(|| path.to_string()),
    ));
    ROUTER.with(|router| {
        *router.borrow_mut() = Some(Mounted {
            table,
            not_found,
            history,
        });
    });

    crate::render::mount_app(screen, host)
}

/// Go to `path` on the next frame.
///
/// Safe from a worker thread's posted job, and safe to call twice in one
/// frame: it writes a value, and the frame draws what the value became.
pub fn navigate(path: impl Into<String>) {
    let path = path.into();
    let mounted = ROUTER.with(|router| {
        if let Some(mounted) = router.borrow_mut().as_mut() {
            mounted.history.push(path.clone());
            return true;
        }
        false
    });
    if !mounted {
        // Nobody is listening yet. `start` will ask.
        ASKED_FOR.with(|held| *held.borrow_mut() = Some(path));
        return;
    }
    store::<Current>().set(Current(path));
}

/// Go back to the screen before this one. `false` if there is none.
pub fn back() -> bool {
    let previous = ROUTER.with(|router| {
        let mut router = router.borrow_mut();
        let mounted = router.as_mut()?;
        if mounted.history.len() < 2 {
            return None;
        }
        mounted.history.pop();
        mounted.history.last().cloned()
    });

    match previous {
        Some(path) => {
            store::<Current>().set(Current(path));
            true
        }
        None => false,
    }
}

/// The path the application is on.
pub fn current() -> String {
    store::<Current>().get().0
}

/// The one component the router mounts.
///
/// Reading the store here is what enrols this component as its reader,
/// so a `navigate` marks exactly this instance for the next frame.
fn screen() -> Element {
    let path = store::<Current>().get().0;

    // Taken out before anything is rendered: a page's body is the
    // developer's code and may well call `navigate`, which borrows this
    // same cell.
    let Some((table, not_found)) = ROUTER.with(|router| {
        router
            .borrow()
            .as_ref()
            .map(|mounted| (mounted.table, mounted.not_found))
    }) else {
        return Element::Empty;
    };

    match match_route(table, &path) {
        Some((route, params)) => wrap(route, &params),
        // A path nobody declared is a screen, not a blank window.
        None => rustx_dom::state::with_key("not-found", || {
            component_at(
                Slot {
                    file: file!(),
                    line: line!(),
                    column: column!(),
                },
                not_found,
            )
        }),
    }
}

/// The page inside its layouts, outermost first.
///
/// Everything here turns on the key each one is rendered under, because
/// the key is what makes two renders the same component or two different
/// ones.
///
/// The page is keyed by its **pattern**. `/device/7` to `/device/8` is
/// the same key, so the same instance: the page re-renders with the new
/// prop and an effect watching the id re-runs, closing before opening.
/// `/` to `/device/7` is a different key, so a different instance: the
/// screen that was left is not reached by this render at all, and the
/// end of the render releases it — with the tasks its cells held.
///
/// A layout is keyed by which layout it is, and how deep. A layout that
/// encloses both the old screen and the new one is therefore reached
/// under the same key and keeps its cells — which is the whole comfort
/// of nested layouts, and the reason a navigation bar does not start
/// counting again when the screen under it changes.
fn wrap(route: &Route, params: &Params) -> Element {
    let page = rustx_dom::state::with_key(route.pattern, || {
        component_at(
            Slot {
                file: file!(),
                line: line!(),
                column: column!(),
            },
            || (route.render)(params),
        )
    });

    let mut drawn = page;
    for (depth, layout) in route.layouts.iter().enumerate().rev() {
        let inner = drawn;
        let which = *layout as usize;
        drawn = rustx_dom::state::with_key(format!("{depth}-{which}"), || {
            component_at(
                Slot {
                    file: file!(),
                    line: line!(),
                    column: column!(),
                },
                || layout(inner),
            )
        });
    }
    drawn
}
