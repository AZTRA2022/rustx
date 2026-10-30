//! A component written inside another one.
//!
//! A layout is a component that calls another component, so this is what
//! layouts are built on. The rule is the one cells already follow: the
//! place the call is written identifies it, and what stops being reached
//! is released.

use std::cell::Cell;
use std::rc::Rc;

use rustx_dom::state::{
    InstanceId, Scope, Slot, cell, children_of, component_at, current, drop_instance, end_render,
    live_instances, next_instance,
};

macro_rules! here {
    () => {
        Slot {
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

/// Says when it was released — a camera, a file, a capture session.
struct Noisy(Rc<Cell<bool>>);

impl Drop for Noisy {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

thread_local! {
    /// The parent of these tests, taken from the one counter the
    /// framework allocates instances from — a hardcoded number could
    /// collide with a child's, and a child is allocated from there too.
    static PARENT_ID: std::cell::OnceCell<InstanceId> = const { std::cell::OnceCell::new() };
}

fn parent() -> InstanceId {
    PARENT_ID.with(|id| *id.get_or_init(next_instance))
}

#[test]
fn the_same_component_written_twice_is_two_instances() {
    // Review Focus 2: one line of code, two components, two states.
    let _scope = Scope::enter(parent());
    let first = component_at(here!(), || current().expect("inside a child"));
    let second = component_at(here!(), || current().expect("inside a child"));
    assert_ne!(first, second);
}

#[test]
fn a_child_keeps_its_cells_across_renders_of_its_parent() {
    assert_eq!(counts_over_two_renders_of_the_same_parent(), [1, 2]);
}

#[test]
fn a_child_that_stops_being_reached_is_released() {
    // Review Focus 1, the half that lives in `rustx-dom`: a page the
    // router no longer renders is released because nothing reached it.
    let witness = Rc::new(Cell::new(false));
    render_parent(Some(Rc::clone(&witness)));
    assert!(!witness.get(), "the child was released while still rendered");

    render_parent(None);
    assert!(witness.get());
}

#[test]
fn releasing_a_parent_releases_the_child_it_declared() {
    let witness = Rc::new(Cell::new(false));
    render_parent(Some(Rc::clone(&witness)));

    drop_instance(parent());
    assert!(witness.get());
}

#[test]
fn a_childs_cells_are_not_its_parents() {
    let _scope = Scope::enter(parent());
    cell(here!(), || 1_u8);
    let inside = component_at(here!(), || {
        cell(here!(), || 2_u8);
        live_instances()
    });
    assert_eq!(inside, 2, "the child should have cells of its own");
}

#[test]
fn a_parent_knows_the_children_it_declared() {
    let child = {
        let _scope = Scope::enter(parent());
        let child = component_at(here!(), || current().expect("inside a child"));
        end_render().expect("the parent reaches everything it declares");
        child
    };
    assert_eq!(children_of(parent()), vec![child]);
}

/// A parent whose child counts the renders it has had of its own.
fn counts_over_two_renders_of_the_same_parent() -> [u32; 2] {
    let slot = here!();
    let mut counts = [0; 2];
    for count in &mut counts {
        let _scope = Scope::enter(parent());
        *count = component_at(slot, || {
            let renders = cell(
                Slot {
                    file: file!(),
                    line: 1,
                    column: 1,
                },
                || 0_u32,
            );
            renders.update(|value| *value += 1);
            renders.get()
        });
        end_render().expect("the parent reaches everything it declares");
    }
    counts
}

/// One render of a parent that declares a child only when asked to —
/// the shape of a router rendering one page and then another.
fn render_parent(with_child: Option<Rc<Cell<bool>>>) {
    let slot = here!();
    let _scope = Scope::enter(parent());
    if let Some(witness) = with_child {
        component_at(slot, || {
            let kept = rustx_dom::state::keep_at(
                Slot {
                    file: file!(),
                    line: 2,
                    column: 2,
                },
                || Noisy(witness),
            );
            kept.with(|_| ());
        });
    }
    end_render().expect("a parent may declare a different child each render");
}

#[test]
fn a_childs_keyed_rows_are_its_own_and_not_the_key_it_was_rendered_under() {
    // I1 of the plan 5 review. The router renders every page inside
    // `with_key(pattern)`, so inside the page the key path began with
    // the pattern. A row the page then removed was looked for by the
    // outermost key — the pattern — and dropping it would have taken
    // every cell the page held, not the row's.
    use rustx_dom::state::with_key;

    let released = Rc::new(Cell::new(false));

    let render = |rows: &[&str], released: &Rc<Cell<bool>>| {
        let _scope = Scope::enter(parent());
        with_key("/device/[id]", || {
            component_at(here!(), || {
                for row in rows {
                    with_key(*row, || {
                        let kept = rustx_dom::state::keep_at(here!(), || {
                            Noisy(Rc::clone(released))
                        });
                        kept.with(|_| ());
                    });
                }
                // `component_at` closes the child's render itself.
            });
        });
        end_render().expect("the parent reached its own values");
    };

    render(&["a", "b"], &released);
    assert!(!released.get(), "nothing left the list yet");

    // `b` is gone and `a` is still there: `a`'s value has to survive.
    render(&["a"], &released);
    assert!(released.get(), "the row that left should have been released");
    assert!(
        rustx_dom::state::live_instances() > 0,
        "the row that stayed should still hold its value"
    );
}

#[test]
fn a_child_that_left_the_tree_leaves_no_row_behind_it() {
    // I2 of the plan 5 review. The identity map was pruned by parent,
    // so the row that named a released child stayed: one row per key
    // the application had ever shown, for as long as it ran.
    let before = rustx_dom::state::live_child_ids();

    for key in ["a", "b", "c"] {
        let _scope = Scope::enter(parent());
        rustx_dom::state::with_key(key, || {
            component_at(here!(), || {
                let kept = rustx_dom::state::keep_at(here!(), || 1_u8);
                kept.with(|_| ());
            });
        });
        // Nothing was declared under the next key, so the child under
        // the last one left the tree.
        end_render().expect("the parent reached its own values");
    }

    assert_eq!(
        rustx_dom::state::live_child_ids(),
        before + 1,
        "only the child of the last render is still in the tree"
    );
}
