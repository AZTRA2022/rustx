//! The combinations the stability brief names, driven without a window.
//!
//! Every test in this file is one of the six shapes §5 of
//! the runtime is held to:
//! nested components with shared state, navigation while asynchronous
//! tasks are running, effects depending on several reactive values,
//! several routes sharing application-level state, rapid successive
//! state updates, and mount/unmount churn under load.
//!
//! # The harness, and what it is honest about
//!
//! There is no CEF here and no DOM. What a frame does to the DOM is
//! `rustx_engine::render::apply`, and that needs a V8 context; what a
//! frame does to *state* is everything else, and that needs nothing but
//! this thread. So [`Page`] below is `render::render_dirty` with the
//! patching taken out: drain what the futures posted, ask which
//! instances are dirty, render those, close the render. The ordering is
//! the same ordering, deliberately, because the ordering is what these
//! tests are about.
//!
//! What this therefore does **not** cover: that the patches the diff
//! produced were applied, that React was told about a boundary, that a
//! handler is attached to the right node. Those are the scenarios that
//! start a real window through `mod common`, and
//! Which of them a given machine can run depends on its own hardware.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use rustx_dom::Element;
use rustx_dom::route::{Params, Route};
use rustx_dom::state::{
    InstanceId, Scope, Slot, State, apply_remote, cell, component_at, drop_instance, effect_at,
    end_render, keep_at, live_child_ids, live_instances, next_instance, open_remote, root_of,
    take_dirty, with_key,
};
use rustx_dom::store::{Shared, shared};
use rustx_engine::{router, task};

macro_rules! here {
    () => {
        Slot {
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

const PATIENCE: Duration = Duration::from_secs(5);

/// Waits for `condition` rather than sleeping a guessed amount.
fn within(patience: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + patience;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    condition()
}

/// One root, rendered the way a frame renders one.
struct Page<'a> {
    id: InstanceId,
    component: Box<dyn Fn() -> Element + 'a>,
    renders: Cell<u32>,
    complaints: RefCell<Vec<String>>,
}

impl<'a> Page<'a> {
    fn mount(component: impl Fn() -> Element + 'a) -> Self {
        open_remote();
        let _ = take_dirty();
        let page = Self {
            id: next_instance(),
            component: Box::new(component),
            renders: Cell::new(0),
            complaints: RefCell::new(Vec::new()),
        };
        page.render();
        page
    }

    fn render(&self) -> Element {
        let _scope = Scope::enter(self.id);
        let drawn = (self.component)();
        if let Err(complaint) = end_render() {
            self.complaints.borrow_mut().push(complaint);
        }
        self.renders.set(self.renders.get() + 1);
        drawn
    }

    /// One frame: what the futures posted, then what is dirty.
    ///
    /// The same order as `frame::tick` followed by
    /// `render::render_dirty`, which is the order the whole design turns
    /// on — a write that landed has to be in the render it asked for
    /// and not the one after.
    fn frame(&self) -> Option<Element> {
        apply_remote();
        let dirty: Vec<InstanceId> = take_dirty().into_iter().map(root_of).collect();
        if !dirty.contains(&self.id) {
            return None;
        }
        Some(self.render())
    }

    /// Frames until nothing more is dirty, and what the last one drew.
    ///
    /// Bounded, because a component that writes its own state during a
    /// render asks for a render that writes it again — and a test that
    /// looped for ever over it would be a worse way to learn that than
    /// an assertion.
    fn settle(&self) -> Option<Element> {
        let mut last = None;
        for _ in 0..32 {
            match self.frame() {
                Some(drawn) => last = Some(drawn),
                None => return last,
            }
        }
        panic!("this page never settled: it writes its own state while rendering");
    }

    fn renders(&self) -> u32 {
        self.renders.get()
    }

    fn complaints(&self) -> Vec<String> {
        self.complaints.borrow().clone()
    }
}

impl Drop for Page<'_> {
    fn drop(&mut self) {
        drop_instance(self.id);
    }
}

/// Everything the elements of a tree say.
fn words(element: &Element) -> String {
    match element {
        Element::Text(text) => text.clone(),
        Element::Fragment(children) => children.iter().map(words).collect(),
        Element::Tag(tag) => tag.children.iter().map(words).collect(),
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------
// Nested components with shared state.
// ---------------------------------------------------------------------

#[test]
fn nested_components_see_one_shared_value_and_keep_their_own() {
    // A panel inside a page, twice, each counting on its own and all of
    // them reading one application-level value. The value is what
    // "shared" means; the counts are what "their own" means, and the
    // bug this shape finds is the two of them being one cell.
    //
    // The handles each render hands out are kept, so the test can write
    // to a nested component's state the way a handler inside it would —
    // from outside any render, through the cell it found by place.
    #[derive(Default)]
    struct Seen {
        own: Vec<State<u32>>,
        everyones: Vec<u32>,
    }
    let seen: Rc<RefCell<Seen>> = Rc::default();

    let panel = {
        let seen = Rc::clone(&seen);
        move || {
            component_at(here!(), || {
                let own = cell(here!(), || 0u32);
                let everyones = shared("architecture-total", || 0u32);
                let mut seen = seen.borrow_mut();
                seen.own.push(own.clone());
                seen.everyones.push(everyones.get());
                Element::Text(own.get().to_string())
            })
        }
    };

    let page = Page::mount(move || {
        Element::Fragment(vec![with_key("left", &panel), with_key("right", &panel)])
    });

    assert_eq!(
        seen.borrow().everyones,
        vec![0, 0],
        "the two panels did not read one value"
    );

    // A write inside the left panel. The panel is a child, so what has
    // to render is the root above it: that is the step a dirty child
    // used to be thrown away at.
    let left = seen.borrow().own[0].clone();
    seen.borrow_mut().own.clear();
    seen.borrow_mut().everyones.clear();
    left.set(5);
    let drawn = page.settle().expect("the page redrew");

    assert_eq!(
        words(&drawn),
        "50",
        "one panel's own state reached the other: {:?}",
        words(&drawn)
    );
    assert_eq!(
        seen.borrow().everyones,
        vec![0, 0],
        "a write to one panel's own cell changed the shared value"
    );

    // And the shared value reaches both panels, written from outside
    // either of them.
    seen.borrow_mut().own.clear();
    seen.borrow_mut().everyones.clear();
    shared("architecture-total", || 0u32).set(7);
    let drawn = page.settle().expect("a shared write redrew the page");
    assert_eq!(
        seen.borrow().everyones,
        vec![7, 7],
        "a shared value did not reach every reader"
    );
    assert_eq!(
        words(&drawn),
        "50",
        "the panels lost their own state when the shared value changed"
    );

    assert!(
        page.complaints().is_empty(),
        "the render complained: {:?}",
        page.complaints()
    );
}

// ---------------------------------------------------------------------
// Effects depending on several reactive values.
// ---------------------------------------------------------------------

#[test]
fn an_effect_on_two_values_runs_when_either_changes_and_not_otherwise() {
    let runs = Rc::new(Cell::new(0u32));
    let closed = Rc::new(Cell::new(0u32));
    type Both = Rc<RefCell<Option<(State<u32>, State<u32>)>>>;
    let handles: Both = Rc::default();

    /// What an effect that opens something hands back. Dropping it is
    /// the closing, which is the whole bargain of `effect`.
    struct Open(Rc<Cell<u32>>);
    impl Drop for Open {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    let page = {
        let runs = Rc::clone(&runs);
        let closed = Rc::clone(&closed);
        let handles = Rc::clone(&handles);
        Page::mount(move || {
            let left = cell(here!(), || 0u32);
            let right = cell(here!(), || 0u32);
            *handles.borrow_mut() = Some((left.clone(), right.clone()));
            // The one value the effect watches is a pair of them, which
            // is how "several reactive values" is written when there is
            // no dependency array to keep in step with the body.
            let runs = Rc::clone(&runs);
            let closed = Rc::clone(&closed);
            effect_at(here!(), (left.get(), right.get()), move || {
                runs.set(runs.get() + 1);
                Open(closed)
            });
            Element::Text(format!("{} {}", left.get(), right.get()))
        })
    };

    assert_eq!(runs.get(), 1, "the effect did not run on the first render");
    assert_eq!(closed.get(), 0, "it closed before anything changed");

    // A render that changes neither value must not run it again.
    // Nothing is dirty, so the page is drawn by hand.
    page.render();
    assert_eq!(runs.get(), 1, "the effect ran again for nothing");
    assert_eq!(closed.get(), 0);

    let (left, right) = handles.borrow().clone().expect("the cells were handed out");

    left.set(1);
    page.settle().expect("a change redraws");
    assert_eq!(runs.get(), 2, "a change to the first value was missed");
    assert_eq!(closed.get(), 1, "it opened twice without closing once");

    right.set(1);
    page.settle().expect("a change redraws");
    assert_eq!(runs.get(), 3, "a change to the second value was missed");
    assert_eq!(closed.get(), 2);

    // Writing the same values again changes nothing, which is what
    // "when either changes" means.
    left.set(1);
    page.settle();
    assert_eq!(runs.get(), 3, "the effect ran for a value that is the same");

    // Leaving the tree closes what the last run left open, with nothing
    // written about closing it.
    drop(page);
    assert_eq!(
        closed.get(),
        3,
        "an effect's guard outlived the component that opened it"
    );
}

// ---------------------------------------------------------------------
// Rapid successive state updates.
// ---------------------------------------------------------------------

#[test]
fn ten_thousand_writes_from_a_future_cost_one_render() {
    // The promise of the threading model: the writes all land, the
    // screen is written once. A loop pushing as fast as it can must not
    // be able to drown the renderer, and there is no debounce to
    // configure.
    const MANY: u32 = 10_000;

    let held: Rc<RefCell<Option<State<u32>>>> = Rc::default();
    let page = {
        let held = Rc::clone(&held);
        Page::mount(move || {
            let count = cell(here!(), || 0u32);
            *held.borrow_mut() = Some(count.clone());
            Element::Text(count.get().to_string())
        })
    };

    let count = held.borrow().clone().expect("the cell was handed out");
    let remote = count.remote();
    let before = page.renders();

    let _task = task::spawn_future(async move {
        for _ in 0..MANY {
            remote.update(|count| *count += 1);
        }
    });

    assert!(
        within(PATIENCE, || {
            apply_remote();
            count.get() == MANY
        }),
        "only {} of {MANY} writes landed",
        count.get()
    );

    // Every write marked the instance; the marks coalesce, so what
    // follows is one render and not ten thousand.
    page.settle();
    let cost = page.renders() - before;
    assert!(
        cost <= 2,
        "{MANY} writes cost {cost} renders; coalescing is the whole promise"
    );
    assert_eq!(words(&page.render()), MANY.to_string());
}

// ---------------------------------------------------------------------
// Mount and unmount churn under load.
// ---------------------------------------------------------------------

#[test]
fn mounting_and_releasing_many_components_leaves_nothing_behind() {
    // The number that matters is not how fast this is: it is that both
    // counts come back to where they started. A row kept per key ever
    // shown, or an instance whose cells are never released, is a leak
    // that only shows up in an application somebody has left open all
    // day.
    const CHURN: usize = 2_000;

    let instances = live_instances();
    let identities = live_child_ids();

    for round in 0..CHURN {
        let page = Page::mount(|| {
            with_key(format!("row-{round}"), || {
                component_at(here!(), || {
                    let _mine = cell(here!(), || vec![0u8; 64]);
                    Element::Empty
                })
            })
        });
        page.frame();
        drop(page);
        // Checked along the way as well as at the end, so that a
        // failure names the round it first grew on.
        if round % 500 == 0 {
            assert_eq!(
                live_instances(),
                instances,
                "instances grew by round {round}"
            );
            assert_eq!(
                live_child_ids(),
                identities,
                "child identities grew by round {round}"
            );
        }
    }

    assert_eq!(
        live_instances(),
        instances,
        "{CHURN} mountings left instances behind"
    );
    assert_eq!(
        live_child_ids(),
        identities,
        "{CHURN} mountings left child identities behind"
    );
}

#[test]
fn a_task_a_component_kept_stops_when_the_component_churns_out() {
    // The same churn, with something that has to be closed in each
    // round: if releasing a component did not drop what it kept, this
    // would leave two hundred threads behind.
    const CHURN: usize = 200;
    let running = Arc::new(AtomicU32::new(0));

    for _ in 0..CHURN {
        let running = Arc::clone(&running);
        let page = Page::mount(move || {
            let running = Arc::clone(&running);
            keep_at(here!(), move || {
                running.fetch_add(1, Ordering::AcqRel);
                task::spawn(move |alive| {
                    while alive.get() {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    running.fetch_sub(1, Ordering::AcqRel);
                })
            });
            Element::Empty
        });
        drop(page);
    }

    assert!(
        within(Duration::from_secs(60), || running.load(Ordering::Acquire)
            == 0),
        "{} of {CHURN} tasks were still running after their components left",
        running.load(Ordering::Acquire)
    );
}

// ---------------------------------------------------------------------
// Routing.
// ---------------------------------------------------------------------
//
// One test, not four. Which thread the router was mounted on is a
// process-wide answer — `current()` asked from anywhere else reads the
// copy a future reads — so two tests installing a router in one test
// binary would be two threads arguing about where the application is.

fn basket() -> Shared<Vec<String>> {
    shared("architecture-basket", Vec::new)
}

fn shop(_: &Params) -> Element {
    let seen = cell(here!(), || 0u32);
    Element::Text(format!("shop {} {}", basket().get().len(), seen.get()))
}

fn till(_: &Params) -> Element {
    Element::Text(format!("till {}", basket().get().join(",")))
}

/// A task started by the page at `/watch`, kept in one of its cells.
static STARTED: AtomicBool = AtomicBool::new(false);
static STOPPED: AtomicBool = AtomicBool::new(false);

fn watching(_: &Params) -> Element {
    keep_at(here!(), || {
        STARTED.store(true, Ordering::Release);
        task::spawn(|alive| {
            while alive.get() {
                std::thread::sleep(Duration::from_millis(1));
            }
            STOPPED.store(true, Ordering::Release);
        })
    });
    Element::Text("watching".to_string())
}

/// A bar above every screen, which must keep its own state across a
/// navigation: that is the whole comfort of a nested layout.
fn bar(inner: Element, _: &Params) -> Element {
    let drawn = cell(here!(), || 0u32);
    Element::Fragment(vec![Element::Text(format!("[{}]", drawn.get())), inner])
}

static TABLE: &[Route] = &[
    Route {
        pattern: "/",
        render: shop,
        layouts: &[bar],
        loading: None,
        failed: None,
    },
    Route {
        pattern: "/till",
        render: till,
        layouts: &[bar],
        loading: None,
        failed: None,
    },
    Route {
        pattern: "/watch",
        render: watching,
        layouts: &[bar],
        loading: None,
        failed: None,
    },
];

#[test]
fn navigation_keeps_what_stays_and_lets_go_of_what_leaves() {
    router::install(TABLE, "/", router::Screens::default());
    let page = Page::mount(router::screen);

    assert_eq!(words(&page.render()), "[0]shop 0 0");

    // Several routes sharing application-level state: the first screen
    // writes it, the second reads it.
    basket().update(|held| held.push("coffee".to_string()));
    page.settle();
    assert_eq!(words(&page.render()), "[0]shop 1 0");

    router::navigate("/till");
    let drawn = page.settle().expect("a navigation redraws the screen");
    assert_eq!(
        words(&drawn),
        "[0]till coffee",
        "the value the first screen wrote did not survive the navigation"
    );
    assert_eq!(router::current(), "/till");

    // Navigation while an asynchronous task is running. Nothing in the
    // page says anything about stopping it: the token is one of the
    // page's cells, the navigation means the page is not reached by the
    // next render, and the end of that render releases it.
    router::navigate("/watch");
    page.settle();
    assert!(STARTED.load(Ordering::Acquire), "the page never started it");
    assert!(
        !STOPPED.load(Ordering::Acquire),
        "it stopped while its page was still on screen"
    );

    router::navigate("/till");
    page.settle();
    assert!(
        within(PATIENCE, || STOPPED.load(Ordering::Acquire)),
        "the application navigated away and the task kept running"
    );

    // Going back is still a navigation, and the history is the
    // application's own.
    assert!(router::can_go_back());
    assert!(router::back());
    page.settle();
    assert_eq!(router::route(), "/watch");

    // A path nobody declared is the engine's own screen rather than a
    // blank window.
    router::navigate("/nothing/here");
    let drawn = page.settle().expect("a navigation redraws the screen");
    assert!(
        words(&drawn).contains("does not exist"),
        "got {:?}",
        words(&drawn)
    );
}
