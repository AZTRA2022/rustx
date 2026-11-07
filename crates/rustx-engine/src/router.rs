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
    /// Where the application has been, and where it is in that. The
    /// window's own history is the OS layer's business; this is the
    /// application's.
    history: History,
}

/// Where the application has been, and where in it it stands.
///
/// A list and a finger on it, which is what a history is: going back
/// moves the finger and keeps what is in front, so going forward is
/// possible; going somewhere new drops what was in front, because that
/// future did not happen. Exactly a browser's rule, and the reason
/// `forward` could not exist while this was a stack that only pushed
/// and popped.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct History {
    paths: Vec<String>,
    at: usize,
}

impl History {
    fn from(path: &str) -> Self {
        Self {
            paths: vec![path.to_string()],
            at: 0,
        }
    }

    fn here(&self) -> &str {
        self.paths.get(self.at).map(String::as_str).unwrap_or("/")
    }

    /// Somewhere new: what was in front of us never happened.
    fn go(&mut self, path: &str) {
        self.paths.truncate(self.at + 1);
        self.paths.push(path.to_string());
        self.at = self.paths.len() - 1;
    }

    /// Somewhere new *instead of* here: nothing is added, so going back
    /// goes to what was before this screen and not to this screen.
    fn instead(&mut self, path: &str) {
        if self.paths.is_empty() {
            self.paths.push(path.to_string());
            self.at = 0;
            return;
        }
        self.paths[self.at] = path.to_string();
    }

    fn can_go_back(&self) -> bool {
        self.at > 0
    }

    fn can_go_forward(&self) -> bool {
        self.at + 1 < self.paths.len()
    }

    fn back(&mut self) -> Option<&str> {
        if !self.can_go_back() {
            return None;
        }
        self.at -= 1;
        Some(self.here())
    }

    fn forward(&mut self) -> Option<&str> {
        if !self.can_go_forward() {
            return None;
        }
        self.at += 1;
        Some(self.here())
    }
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
    let mut history = History::from(path);
    if let Some(asked_for) = ASKED_FOR.with(|held| held.borrow_mut().take()) {
        history.go(&asked_for);
    }
    let here = history.here().to_string();
    // So that `rustx dev` can open the next window on this screen.
    rustx_dom::carried::be_at(&here);
    store::<Current>().set(Current(here));
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
            mounted.history.go(&path);
            return true;
        }
        false
    });
    if !mounted {
        // Nobody is listening yet. `start` will ask.
        ASKED_FOR.with(|held| *held.borrow_mut() = Some(path));
        return;
    }
    show(&path);
}

/// Go to `path` **instead of** this screen: it takes its place in the
/// history rather than being added after it.
///
/// What a redirection is, and what a form that has just been sent
/// wants: going back from the screen that follows must not come back to
/// the form and send it again.
pub fn replace(path: impl Into<String>) {
    let path = path.into();
    let mounted = ROUTER.with(|router| {
        if let Some(mounted) = router.borrow_mut().as_mut() {
            mounted.history.instead(&path);
            return true;
        }
        false
    });
    if !mounted {
        ASKED_FOR.with(|held| *held.borrow_mut() = Some(path));
        return;
    }
    show(&path);
}

/// Go back to the screen before this one. `false` if there is none.
pub fn back() -> bool {
    let previous = ROUTER.with(|router| {
        let mut router = router.borrow_mut();
        let mounted = router.as_mut()?;
        mounted.history.back().map(str::to_string)
    });
    match previous {
        Some(path) => {
            show(&path);
            true
        }
        None => false,
    }
}

/// Go forward again, to a screen we came back from. `false` if there is
/// none — which is the case as soon as one has gone somewhere new.
pub fn forward() -> bool {
    let next = ROUTER.with(|router| {
        let mut router = router.borrow_mut();
        let mounted = router.as_mut()?;
        mounted.history.forward().map(str::to_string)
    });
    match next {
        Some(path) => {
            show(&path);
            true
        }
        None => false,
    }
}

/// Whether there is anywhere to go back to.
///
/// What a "retour" button asks before it draws itself grey. Until this
/// existed, the only way to know was to call `back` and read what it
/// gave back — that is, to have already gone.
pub fn can_go_back() -> bool {
    ROUTER.with(|router| {
        router
            .borrow()
            .as_ref()
            .is_some_and(|mounted| mounted.history.can_go_back())
    })
}

/// And whether there is anywhere to go forward to.
pub fn can_go_forward() -> bool {
    ROUTER.with(|router| {
        router
            .borrow()
            .as_ref()
            .is_some_and(|mounted| mounted.history.can_go_forward())
    })
}

/// Where the application has been, oldest first, and where it stands in
/// that list.
pub fn history() -> (Vec<String>, usize) {
    ROUTER.with(|router| {
        router
            .borrow()
            .as_ref()
            .map(|mounted| (mounted.history.paths.clone(), mounted.history.at))
            .unwrap_or_default()
    })
}

/// Put this path on the screen, and tell `rustx dev` where we are.
fn show(path: &str) {
    rustx_dom::carried::be_at(path);
    store::<Current>().set(Current(path.to_string()));
}

/// The path the application is on, query string included.
pub fn current() -> String {
    store::<Current>().get().0
}

/// The path alone, without what follows a `?`.
pub fn route() -> String {
    let here = current();
    rustx_dom::route::split_query(&here).0.to_string()
}

/// What the query string of the current path says under `name`, or `""`.
///
/// `/depots?q=rust` gives `query("q") == "rust"`. A query key belongs to
/// the request and not to the route, so no page declares it as a
/// parameter: it is read here, by whoever wants it, layout included.
pub fn query(name: &str) -> String {
    let here = current();
    let (_, query) = rustx_dom::route::split_query(&here);
    rustx_dom::route::query_of(query, name)
}

/// Whether the current path's query string holds this key at all —
/// `?brut` says nothing and is still there.
pub fn has_query(name: &str) -> bool {
    let here = current();
    let (_, query) = rustx_dom::route::split_query(&here);
    rustx_dom::route::has_query(query, name)
}

/// The same path with `name` set to `value` in its query string, or
/// without it when `value` is `None`.
///
/// Written here because a page that wants "the same screen, sorted
/// differently" would otherwise rebuild the URL by hand, and the one
/// thing nobody should have to write by hand is percent-encoding.
pub fn with_query(name: &str, value: Option<&str>) -> String {
    let here = current();
    rustx_dom::route::with_query(&here, name, value)
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

    // A page that is waiting for a `.await` renders nothing — which is
    // right, and is also a blank screen. The `loading.rsx` nearest to
    // it goes there instead, inside the same layouts, so the bar above
    // stays and only the middle says "un instant".
    //
    // Keyed apart from the page: they are two different components, and
    // one must not inherit the other's cells.
    let page = match (&page, route.loading) {
        (Element::Empty, Some(loading)) => rustx_dom::state::with_key(
            format!("loading:{}", route.pattern),
            || {
                component_at(
                    Slot {
                        file: file!(),
                        line: line!(),
                        column: column!(),
                    },
                    loading,
                )
            },
        ),
        _ => page,
    };

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
                || layout(inner, params),
            )
        });
    }
    drawn
}

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn going_back_leaves_somewhere_to_go_forward_to() {
        let mut history = History::from("/");
        history.go("/depots");
        history.go("/depots/rust-lang/rust");
        assert!(history.can_go_back());
        assert!(!history.can_go_forward());

        assert_eq!(history.back(), Some("/depots"));
        assert!(history.can_go_forward());
        assert_eq!(history.forward(), Some("/depots/rust-lang/rust"));
        assert!(!history.can_go_forward());
    }

    #[test]
    fn going_somewhere_new_drops_the_future_that_did_not_happen() {
        let mut history = History::from("/");
        history.go("/a");
        history.go("/b");
        history.back();
        // On est sur `/a`, et `/b` est devant. Aller ailleurs efface
        // `/b` : c'est la règle d'un navigateur, et sans elle « suivant »
        // emmènerait sur un écran dont on vient de se détourner.
        history.go("/c");
        assert!(!history.can_go_forward());
        assert_eq!(history.paths, vec!["/", "/a", "/c"]);
        assert_eq!(history.here(), "/c");
    }

    #[test]
    fn replacing_takes_the_place_of_this_screen() {
        let mut history = History::from("/");
        history.go("/formulaire");
        history.instead("/merci");
        // Revenir ne ramène pas au formulaire qu'on vient de quitter :
        // c'est tout l'intérêt, un formulaire renvoyé deux fois est
        // envoyé deux fois.
        assert_eq!(history.here(), "/merci");
        assert_eq!(history.back(), Some("/"));
        assert_eq!(history.paths, vec!["/", "/merci"]);
    }

    #[test]
    fn the_first_screen_has_nowhere_to_go() {
        let mut history = History::from("/");
        assert!(!history.can_go_back());
        assert!(!history.can_go_forward());
        assert_eq!(history.back(), None);
        assert_eq!(history.forward(), None);
        assert_eq!(history.here(), "/");
    }
}
