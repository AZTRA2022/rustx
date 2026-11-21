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
use std::sync::{Mutex, OnceLock, RwLock};
use std::thread::ThreadId;

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
}

/// Where the application asked to go before there was a router to take
/// it there.
///
/// A script runs when the JavaScript context is created, and the router
/// mounts on the first frame where its host element exists — which is
/// later. A navigation asked for in between used to be overwritten by
/// the starting path: the screen never came up and nothing said why. It
/// is held instead, and honoured on start.
///
/// Not a thread-local, though it was. A `task(async { … })` started
/// from `main` runs on a runtime worker, and the router is not mounted
/// there and never will be: the ask was written into *that* thread's
/// own cell and nobody ever looked at it again. One place, for whoever
/// asks first.
fn asked_for() -> &'static Mutex<Option<String>> {
    static ASKED_FOR: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    ASKED_FOR.get_or_init(|| Mutex::new(None))
}

/// Hold a navigation until there is a router to honour it.
fn ask_for(path: String) {
    if let Ok(mut held) = asked_for().lock() {
        *held = Some(path);
    }
}

/// What was asked for before the router started, if anything.
fn take_what_was_asked_for() -> Option<String> {
    asked_for().lock().ok().and_then(|mut held| held.take())
}

/// Where the application is, readable from any thread.
///
/// The router lives on the thread that draws: its table, its history
/// and the store that holds the current path are all thread-locals, and
/// a future runs somewhere else. `current()` answered `"/"` there, so
/// `with_query("q", …)` built a path from the wrong place and
/// `replace(…)` went into a thread-local nobody would ever read — a
/// search that wrote its term into the address wrote it nowhere, in
/// silence. This is the copy a future reads, written on the page thread
/// every time the history changes.
#[derive(Default, Clone)]
struct Elsewhere {
    here: String,
    paths: Vec<String>,
    at: usize,
}

fn elsewhere() -> &'static RwLock<Elsewhere> {
    static ELSEWHERE: OnceLock<RwLock<Elsewhere>> = OnceLock::new();
    ELSEWHERE.get_or_init(|| RwLock::new(Elsewhere::default()))
}

/// The thread the router was mounted on: the one that draws.
static PAGE: OnceLock<ThreadId> = OnceLock::new();

/// Is this the thread the screen is drawn on?
///
/// Unknown before the router has started, and then it is this one: a
/// navigation asked for before there is a router to take it is held by
/// `ASKED_FOR`, which is where it was already held.
fn on_the_page_thread() -> bool {
    match PAGE.get() {
        Some(page) => *page == std::thread::current().id(),
        None => true,
    }
}

/// Take the copy a future reads from the router on this thread.
fn remember() {
    let snapshot = ROUTER.with(|router| {
        router.borrow().as_ref().map(|mounted| Elsewhere {
            here: mounted.history.here().to_string(),
            paths: mounted.history.paths.clone(),
            at: mounted.history.at,
        })
    });
    if let Some(snapshot) = snapshot
        && let Ok(mut held) = elsewhere().write()
    {
        *held = snapshot;
    }
}

/// What a reader on another thread gets.
fn remembered() -> Elsewhere {
    elsewhere()
        .read()
        .map(|held| held.clone())
        .unwrap_or_default()
}

/// Run this call again on the page thread, at the start of the next
/// frame.
///
/// `false` when there is no window to run it on any more, which is the
/// one case where dropping it is right.
fn on_the_next_frame(job: impl FnOnce() + Send + 'static) -> bool {
    crate::frame::poster().post(job).is_ok()
}

/// The two screens the router falls back to.
///
/// A path nobody declared and a page that fell over are the two ways an
/// application can end up with nothing to draw, and neither may be a
/// blank window. Both have an answer the engine brings itself; an
/// application that wants its own says so.
#[derive(Clone, Copy)]
pub struct Screens {
    /// Shown for a path no route matches.
    pub not_found: fn() -> Element,
    /// Shown when drawing a page panicked, and given what it said.
    ///
    /// Only reached where the page has no `error.rsx` of its own: the
    /// nearest one above it wins, because a frontier at a segment is
    /// the whole point of writing one.
    pub failed: fn(String) -> Element,
}

impl Default for Screens {
    fn default() -> Self {
        Self {
            not_found: no_such_screen,
            failed: fell_over,
        }
    }
}

struct Mounted {
    table: &'static [Route],
    screens: Screens,
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

/// How many screens back the history remembers.
///
/// A list that only ever grew is a leak, and a slow one: an application
/// whose panel switches tab on a timer, or whose list writes its filter
/// into the address on every keystroke, adds a `String` per navigation
/// for as long as the window is open. Nothing ever reads the far end of
/// it — a browser keeps about fifty entries per tab for the same reason
/// — so the oldest screens are dropped and the finger moves with them.
const MOST: usize = 256;

impl History {
    fn from(path: &str) -> Self {
        Self {
            paths: vec![path.to_string()],
            at: 0,
        }
    }

    /// Drop the oldest screens, keeping the finger on the same one.
    fn forget_the_oldest(&mut self) {
        if self.paths.len() <= MOST {
            return;
        }
        let too_many = self.paths.len() - MOST;
        self.paths.drain(..too_many);
        self.at = self.at.saturating_sub(too_many);
    }

    fn here(&self) -> &str {
        self.paths.get(self.at).map(String::as_str).unwrap_or("/")
    }

    /// Somewhere new: what was in front of us never happened.
    fn go(&mut self, path: &str) {
        self.paths.truncate(self.at + 1);
        self.paths.push(path.to_string());
        self.at = self.paths.len() - 1;
        self.forget_the_oldest();
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
///
/// The screen for a page that fell over is the engine's own. Use
/// [`start_with`] to give that one too.
pub fn start(
    table: &'static [Route],
    host: &str,
    path: &str,
    not_found: fn() -> Element,
) -> Result<(), DomError> {
    start_with(
        table,
        host,
        path,
        Screens {
            not_found,
            ..Screens::default()
        },
    )
}

/// The same, with both fallback screens named.
pub fn start_with(
    table: &'static [Route],
    host: &str,
    path: &str,
    screens: Screens,
) -> Result<(), DomError> {
    // Asked again on every frame until it works. `install` takes what the
    // page asked for while there was no router, so it must not run
    // before there is a page to mount into: the next frame would find
    // that request already used up.
    if crate::dom::Document::current().is_none() {
        return Err(DomError::NoContext);
    }
    install(table, path, screens);
    crate::render::mount_app(screen, host)
}

/// Give this thread the table and the starting path, and mount nothing.
///
/// What [`start_with`] does before it mounts, which is everything that
/// does not need a window: the table, the history, the store the screen
/// is read out of, and which thread draws. [`screen`] then answers with
/// the element the router would put on the page.
///
/// It is public for two callers. The first is a test: routing is the
/// part of the engine with the most behaviour and the least need for a
/// DOM — which screen a path names, what a layout keeps across a
/// navigation, which instance is released — and all of it can be
/// checked in a process with no window at all. The second is an
/// application that places the router's output itself rather than
/// letting it mount under an id.
pub fn install(table: &'static [Route], path: &str, screens: Screens) {
    // Where the application has been is the starting path, and then
    // wherever it asked to go while it was waiting for us.
    let mut history = History::from(path);
    if let Some(asked_for) = take_what_was_asked_for() {
        history.go(&asked_for);
    }
    let here = history.here().to_string();
    // So that `rustx dev` can open the next window on this screen.
    rustx_dom::carried::be_at(&here);
    store::<Current>().set(Current(here));
    ROUTER.with(|router| {
        *router.borrow_mut() = Some(Mounted {
            table,
            screens,
            history,
        });
    });

    let _ = PAGE.set(std::thread::current().id());
    remember();
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
        // Off the thread that draws — a future, a worker — where the
        // router is a thread-local that is not there. The call is run
        // again where it means something, instead of being written into
        // a cell nobody reads.
        if !on_the_page_thread() {
            on_the_next_frame(move || navigate(path));
            return;
        }
        // Nobody is listening yet. `start` will ask.
        ask_for(path);
        return;
    }
    show(&path);
    remember();
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
        if !on_the_page_thread() {
            on_the_next_frame(move || replace(path));
            return;
        }
        ask_for(path);
        return;
    }
    show(&path);
    remember();
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
            remember();
            true
        }
        // Off the page thread there is no history to ask, so the
        // question is asked again where it can be answered. What comes
        // back says the request was accepted and not that there was
        // somewhere to go: a future cannot be told the answer to a
        // question that will be asked after it has moved on.
        None if !on_the_page_thread() => on_the_next_frame(|| {
            back();
        }),
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
            remember();
            true
        }
        None if !on_the_page_thread() => on_the_next_frame(|| {
            forward();
        }),
        None => false,
    }
}

/// Whether there is anywhere to go back to.
///
/// What a "retour" button asks before it draws itself grey. Until this
/// existed, the only way to know was to call `back` and read what it
/// gave back — that is, to have already gone.
pub fn can_go_back() -> bool {
    if !on_the_page_thread() {
        return remembered().at > 0;
    }
    ROUTER.with(|router| {
        router
            .borrow()
            .as_ref()
            .is_some_and(|mounted| mounted.history.can_go_back())
    })
}

/// And whether there is anywhere to go forward to.
pub fn can_go_forward() -> bool {
    if !on_the_page_thread() {
        let held = remembered();
        return held.at + 1 < held.paths.len();
    }
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
    if !on_the_page_thread() {
        let held = remembered();
        return (held.paths, held.at);
    }
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
    if on_the_page_thread() {
        return store::<Current>().get().0;
    }
    remembered().here
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

/// The one component the router mounts: the screen the application is
/// on, inside its layouts.
///
/// Reading the store here is what enrols the component that calls it as
/// the store's reader, so a `navigate` marks exactly that instance for
/// the next frame. Which is why this is a component and not a getter:
/// calling it outside a render answers correctly and enrols nobody.
pub fn screen() -> Element {
    let path = store::<Current>().get().0;

    // Taken out before anything is rendered: a page's body is the
    // developer's code and may well call `navigate`, which borrows this
    // same cell.
    let Some((table, screens)) = ROUTER.with(|router| {
        router
            .borrow()
            .as_ref()
            .map(|mounted| (mounted.table, mounted.screens))
    }) else {
        return Element::Empty;
    };

    draw(table, screens, &path)
}

thread_local! {
    static REDIRECTS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}

/// A path that always shows another screen (`/old` shows `/new`), like the
/// `redirects` of a Next.js config. Declared once, at start-up.
pub fn redirect(from: &str, to: &str) {
    REDIRECTS.with(|list| list.borrow_mut().push((from.to_string(), to.to_string())));
}

/// The path after the redirects, followed up to eight times so that a loop ends.
fn redirected(path: &str) -> String {
    let mut path = path.to_string();
    for _ in 0..8 {
        let (bare, query) = match path.split_once('?') {
            Some((bare, query)) => (bare.to_string(), Some(query.to_string())),
            None => (path.clone(), None),
        };
        let bare = if bare.len() > 1 {
            bare.trim_end_matches('/').to_string()
        } else {
            bare
        };
        let next = REDIRECTS.with(|list| {
            list.borrow()
                .iter()
                .find(|(from, _)| *from == bare)
                .map(|(_, to)| to.clone())
        });
        let Some(next) = next else { break };
        path = match query {
            Some(query) => format!("{next}?{query}"),
            None => next,
        };
    }
    path
}

/// Which screen a path names, drawn.
///
/// Split out of [`screen`] so that it can be exercised without a
/// window: everything above it is the store and the thread-local the
/// router was mounted into, and everything below it is the part that
/// has to be right.
pub(crate) fn draw(table: &[Route], screens: Screens, path: &str) -> Element {
    let path = redirected(path);
    match match_route(table, &path) {
        Some((route, params)) => wrap(route, &params, screens),
        // A path nobody declared is a screen, not a blank window.
        None => rustx_dom::state::with_key("not-found", || {
            component_at(
                Slot {
                    file: file!(),
                    line: line!(),
                    column: column!(),
                },
                screens.not_found,
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
fn wrap(route: &Route, params: &Params, screens: Screens) -> Element {
    let drawing = || {
        rustx_dom::state::with_key(route.pattern, || {
            component_at(
                Slot {
                    file: file!(),
                    line: line!(),
                    column: column!(),
                },
                || (route.render)(params),
            )
        })
    };

    // A page that falls over used to take the window with it: the panic
    // went up to the frame boundary, which caught it and carried on, so
    // what stayed on screen was the *previous* render and the reason was
    // on a console nobody was reading. A frozen-looking window with the
    // explanation somewhere else is the worst of the three outcomes, so
    // the page is always drawn inside a boundary now.
    //
    // Which screen goes there is the application's choice where it made
    // one — the `error.rsx` nearest the page, and everything around the
    // page is still there, which is the whole point of a frontier being
    // at a segment rather than at the window — and the engine's own
    // otherwise. The panic itself is untouched: Rust's hook has already
    // printed it and its backtrace by the time this runs.
    let failed = route.failed.unwrap_or(screens.failed);
    let page = match caught(drawing) {
        Ok(page) => page,
        Err(pourquoi) => rustx_dom::state::with_key(format!("error:{}", route.pattern), || {
            component_at(
                Slot {
                    file: file!(),
                    line: line!(),
                    column: column!(),
                },
                || failed(pourquoi.clone()),
            )
        }),
    };

    // A page that is waiting for a `.await` renders nothing — which is
    // right, and is also a blank screen. The `loading.rsx` nearest to
    // it goes there instead, inside the same layouts, so the bar above
    // stays and only the middle says "un instant".
    //
    // Keyed apart from the page: they are two different components, and
    // one must not inherit the other's cells.
    let page = match (&page, route.loading) {
        (Element::Empty, Some(loading)) => {
            rustx_dom::state::with_key(format!("loading:{}", route.pattern), || {
                component_at(
                    Slot {
                        file: file!(),
                        line: line!(),
                        column: column!(),
                    },
                    loading,
                )
            })
        }
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

/// The id the engine's own fallback screens carry.
///
/// Named, and part of the contract: a scenario, a test or a developer's
/// own diagnostic has to be able to tell "the screen for a path nobody
/// declared" from "a page that happens to say this does not exist".
pub const NO_SUCH_SCREEN_ID: &str = "rustx-no-such-screen";
/// The same, for the screen a page that fell over is replaced by.
pub const FAILED_SCREEN_ID: &str = "rustx-screen-failed";
/// And for the element inside it that holds what went wrong.
pub const FAILED_REASON_ID: &str = "rustx-screen-failed-reason";

/// Where a path nobody declared leads, unless the application says
/// otherwise: the page Next.js shows (its `404`, same markup, same colours,
/// same system font stack, light or dark by the system's preference).
fn no_such_screen() -> Element {
    next_error(
        NO_SUCH_SCREEN_ID,
        Some("404"),
        "This page could not be found.",
        None,
    )
}

/// And where a page that fell over leads, unless the application wrote
/// an `error.rsx` above it: Next.js's "Application error" page, with what
/// the panic said under it, verbatim, because a screen that says "something
/// went wrong" is a screen nobody can act on.
fn fell_over(pourquoi: String) -> Element {
    next_error(
        FAILED_SCREEN_ID,
        None,
        "Application error: an exception has occurred (see the console for more information).",
        Some(pourquoi),
    )
}

/// The `style` Next.js writes into its error pages, kept as it is: the body
/// colours follow `prefers-color-scheme`, and the divider after the status
/// code is a hairline in either.
const NEXT_ERROR_STYLE: &str = "body{color:#000;background:#fff;margin:0}    .next-error-h1{border-right:1px solid rgba(0,0,0,.3)}    @media (prefers-color-scheme:dark){body{color:#fff;background:#000}    .next-error-h1{border-right:1px solid rgba(255,255,255,.3)}}";

/// Next.js's default error screen. Inline styles for the layout (the
/// application most likely to reach this ships no stylesheet at all).
fn next_error(id: &str, code: Option<&str>, message: &str, reason: Option<String>) -> Element {
    let mut line = Element::tag("div").attribute("style", at("display:inline-block"));
    line = line.child(
        Element::tag("h2")
            .attribute(
                "style",
                at("font-size:14px;font-weight:400;line-height:49px;margin:0"),
            )
            .child(Element::Text(message.to_string()))
            .build(),
    );
    let mut row = Element::tag("div").child(
        Element::tag("style")
            .child(Element::Text(NEXT_ERROR_STYLE.to_string()))
            .build(),
    );
    if let Some(code) = code {
        row = row.child(
            Element::tag("h1")
                .attribute("class", at("next-error-h1"))
                .attribute(
                    "style",
                    at("display:inline-block;margin:0 20px 0 0;padding:0 23px 0 0;                        font-size:24px;font-weight:500;vertical-align:top;line-height:49px"),
                )
                .child(Element::Text(code.to_string()))
                .build(),
        );
    }
    row = row.child(line.build());
    if let Some(reason) = reason {
        row = row.child(
            Element::tag("pre")
                .attribute("id", at(FAILED_REASON_ID))
                .attribute(
                    "style",
                    at("margin:12px auto 0;max-width:46rem;padding:0 24px;text-align:left;                        white-space:pre-wrap;font:12px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;                        opacity:.72"),
                )
                .child(Element::Text(reason))
                .build(),
        );
    }
    Element::tag("div")
        .attribute("id", at(id))
        .attribute(
            "style",
            at("font-family:system-ui,\"Segoe UI\",Roboto,Helvetica,Arial,sans-serif,                \"Apple Color Emoji\",\"Segoe UI Emoji\";height:100vh;text-align:center;                display:flex;flex-direction:column;align-items:center;justify-content:center"),
        )
        .child(row.build())
        .build()
}

fn at(text: &str) -> rustx_dom::AttributeValue {
    rustx_dom::AttributeValue::Text(text.to_string())
}

/// Draw, and say what went wrong rather than letting it through.
///
/// Only where the application put an `error.rsx`: without one, a panic
/// goes on its way to the frame boundary, which is where it went
/// before. Catching it everywhere would turn a bug into a screen
/// nobody asked for.
///
/// The cells a half-drawn component left behind are the frame's
/// business, and the guards that hold the key stack and the scope are
/// `Drop`s — written as such for exactly this, since a component body
/// is the developer's code and may panic.
fn caught(drawing: impl FnOnce() -> Element) -> Result<Element, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(drawing)).map_err(|fallen| {
        // What `panic!` was given, whichever of the two shapes it had.
        if let Some(said) = fallen.downcast_ref::<&str>() {
            (*said).to_string()
        } else if let Some(said) = fallen.downcast_ref::<String>() {
            said.clone()
        } else {
            "the page panicked, and said nothing a string could hold".to_string()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_redirect_shows_another_screen_keeps_the_query_and_ends_on_a_loop() {
        redirect("/old", "/new");
        redirect("/a", "/b");
        redirect("/b", "/a");
        assert_eq!(redirected("/old"), "/new");
        assert_eq!(redirected("/old/?x=1"), "/new?x=1");
        assert_eq!(redirected("/other"), "/other");
        // A loop stops after eight hops instead of hanging.
        let end = redirected("/a");
        assert!(end == "/a" || end == "/b");
    }

    use rustx_dom::state::{InstanceId, Scope};

    thread_local! {
        /// An instance per render, so two tests never share a cell.
        static NEXT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    /// One render, as the engine would open it. `draw` renders
    /// components, and a component has to be rendered by something.
    fn drawn(table: &[Route], screens: Screens, path: &str) -> Element {
        let which = NEXT.with(|next| {
            let which = next.get();
            next.set(which + 1);
            which
        });
        let _scope = Scope::enter(InstanceId(70_000 + which));
        super::draw(table, screens, path)
    }

    /// Everything the elements of a tree say, in order.
    fn words(element: &Element) -> String {
        match element {
            Element::Empty => String::new(),
            Element::Text(text) => text.clone(),
            Element::Fragment(children) => children.iter().map(words).collect(),
            Element::Foreign { id, .. } => id.clone(),
            Element::Tag(tag) => tag.children.iter().map(words).collect(),
        }
    }

    /// Every id the tree carries.
    fn ids(element: &Element) -> Vec<String> {
        fn walk(element: &Element, found: &mut Vec<String>) {
            match element {
                Element::Tag(tag) => {
                    for (name, value) in &tag.attributes {
                        if name == "id"
                            && let rustx_dom::AttributeValue::Text(text) = value
                        {
                            found.push(text.clone());
                        }
                    }
                    for child in &tag.children {
                        walk(child, found);
                    }
                }
                Element::Fragment(children) => {
                    for child in children {
                        walk(child, found);
                    }
                }
                _ => {}
            }
        }
        let mut found = Vec::new();
        walk(element, &mut found);
        found
    }

    fn a_page(_: &Params) -> Element {
        Element::Text("the page".to_string())
    }

    fn falls_over(_: &Params) -> Element {
        panic!("the depot does not exist")
    }

    fn its_own_error(pourquoi: String) -> Element {
        Element::tag("section")
            .attribute("id", at("the-pages-own-error"))
            .child(Element::Text(pourquoi))
            .build()
    }

    /// A bar above every screen, which must survive a page falling over.
    fn bar(inner: Element, _: &Params) -> Element {
        Element::tag("div")
            .attribute("id", at("the-bar"))
            .child(Element::Text("navigation".to_string()))
            .child(inner)
            .build()
    }

    static FINE: &[Route] = &[Route {
        pattern: "/",
        render: a_page,
        layouts: &[],
        loading: None,
        failed: None,
    }];

    static DOOMED: &[Route] = &[Route {
        pattern: "/",
        render: falls_over,
        layouts: &[],
        loading: None,
        failed: None,
    }];

    static DOOMED_WITH_ITS_OWN: &[Route] = &[Route {
        pattern: "/",
        render: falls_over,
        layouts: &[],
        loading: None,
        failed: Some(its_own_error),
    }];

    static DOOMED_UNDER_A_BAR: &[Route] = &[Route {
        pattern: "/",
        render: falls_over,
        layouts: &[bar],
        loading: None,
        failed: None,
    }];

    #[test]
    fn a_path_nobody_declared_is_a_screen_and_not_a_blank_window() {
        let screen = drawn(FINE, Screens::default(), "/nowhere/at/all");
        assert!(
            ids(&screen).iter().any(|id| id == NO_SUCH_SCREEN_ID),
            "got {:?}",
            ids(&screen)
        );
        assert!(
            words(&screen).contains("could not be found"),
            "got {:?}",
            words(&screen)
        );
    }

    #[test]
    fn a_route_that_matches_is_drawn_and_not_replaced() {
        let screen = drawn(FINE, Screens::default(), "/");
        assert_eq!(words(&screen), "the page");
    }

    /// The claim the stability brief asks for by name: a default screen
    /// for an error, **with the error displayed**.
    ///
    /// Before this, a page with no `error.rsx` above it let its panic
    /// through to the frame boundary. The boundary caught it and carried
    /// on, so the window kept showing the render *before* the one that
    /// failed and the reason went to a console nobody was reading.
    #[test]
    fn a_page_that_falls_over_is_replaced_by_a_screen_that_says_why() {
        let screen = drawn(DOOMED, Screens::default(), "/");
        let carried = ids(&screen);
        assert!(
            carried.iter().any(|id| id == FAILED_SCREEN_ID),
            "got {carried:?}"
        );
        assert!(
            carried.iter().any(|id| id == FAILED_REASON_ID),
            "got {carried:?}"
        );
        assert!(
            words(&screen).contains("the depot does not exist"),
            "the reason is not on the screen: {:?}",
            words(&screen)
        );
    }

    #[test]
    fn an_error_screen_the_page_wrote_wins_over_the_engines_own() {
        let screen = drawn(DOOMED_WITH_ITS_OWN, Screens::default(), "/");
        let carried = ids(&screen);
        assert!(
            carried.iter().any(|id| id == "the-pages-own-error"),
            "got {carried:?}"
        );
        assert!(
            !carried.iter().any(|id| id == FAILED_SCREEN_ID),
            "both screens were drawn: {carried:?}"
        );
        assert!(words(&screen).contains("the depot does not exist"));
    }

    #[test]
    fn an_error_screen_the_application_named_wins_over_the_engines_own() {
        let screens = Screens {
            failed: its_own_error,
            ..Screens::default()
        };
        let screen = drawn(DOOMED, screens, "/");
        assert!(
            ids(&screen).iter().any(|id| id == "the-pages-own-error"),
            "got {:?}",
            ids(&screen)
        );
    }

    #[test]
    fn what_is_around_a_page_that_fell_over_is_still_on_the_screen() {
        // A frontier at a segment rather than at the window is the whole
        // point: the bar above the page stays, and only the middle says
        // what happened.
        let screen = drawn(DOOMED_UNDER_A_BAR, Screens::default(), "/");
        let carried = ids(&screen);
        assert!(carried.iter().any(|id| id == "the-bar"), "got {carried:?}");
        assert!(
            carried.iter().any(|id| id == FAILED_SCREEN_ID),
            "got {carried:?}"
        );
        assert!(words(&screen).contains("navigation"));
    }

    #[test]
    fn a_history_that_only_ever_grew_is_a_leak() {
        let mut history = History::from("/");
        for which in 0..MOST * 3 {
            history.go(&format!("/screen/{which}"));
        }
        assert_eq!(
            history.paths.len(),
            MOST,
            "the history grew past what it remembers"
        );
        // And the finger is still on the screen we are actually on.
        assert_eq!(history.here(), format!("/screen/{}", MOST * 3 - 1));
        assert!(history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn going_back_still_works_after_the_oldest_screens_are_forgotten() {
        let mut history = History::from("/");
        for which in 0..MOST * 2 {
            history.go(&format!("/screen/{which}"));
        }
        let wanted = format!("/screen/{}", MOST * 2 - 2);
        assert_eq!(history.back(), Some(wanted.as_str()));
    }

    /// A navigation asked for before the router has mounted is held and
    /// honoured on start — from whichever thread asked.
    ///
    /// It was held in a thread-local, so a `task(async { … })` started
    /// from `main` wrote its ask into a runtime worker's own cell and
    /// nobody ever looked there again: the screen never came up, and
    /// nothing said why.
    #[test]
    fn an_early_navigation_from_another_thread_is_not_lost() {
        let _ = take_what_was_asked_for();
        std::thread::spawn(|| super::ask_for("/from/elsewhere".to_string()))
            .join()
            .expect("the thread asked");
        assert_eq!(
            take_what_was_asked_for().as_deref(),
            Some("/from/elsewhere"),
            "the ask was written somewhere nobody reads"
        );
    }

    #[test]
    fn a_page_that_falls_over_says_what_it_said() {
        let caught = super::caught(|| panic!("the depot does not exist"));
        assert_eq!(caught.err().as_deref(), Some("the depot does not exist"));
    }

    #[test]
    fn a_page_that_draws_is_handed_back_untouched() {
        let drawn = super::caught(|| rustx_dom::Element::Text("bonjour".to_string()));
        match drawn {
            Ok(rustx_dom::Element::Text(said)) => assert_eq!(said, "bonjour"),
            Ok(_) => panic!("the element came back as something else"),
            Err(pourquoi) => panic!("nothing fell over, and yet: {pourquoi}"),
        }
    }

    #[test]
    fn a_reason_built_at_runtime_is_carried_too() {
        // `panic!("{}", …)` donne une `String` et pas un `&str` : deux
        // formes, et une seule attrapée aurait laissé la moitié des
        // chutes sans raison.
        let which = 7;
        let caught = super::caught(|| panic!("depot {which} does not exist"));
        assert_eq!(caught.err().as_deref(), Some("depot 7 does not exist"));
    }

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
