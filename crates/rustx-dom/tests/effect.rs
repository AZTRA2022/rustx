//! Work that happens again only when what it watches has changed.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rustx_dom::state::{
    InstanceId, Scope, Slot, cell, drop_instance, effect_at, end_render, take_dirty,
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

/// What an effect opens: a thing that says when it is closed, which is
/// the shape of a camera, a file or a subscription.
struct Camera {
    id: u32,
    log: Rc<RefCell<Vec<String>>>,
}

impl Drop for Camera {
    fn drop(&mut self) {
        self.log.borrow_mut().push(format!("close {}", self.id));
    }
}

const ONE: InstanceId = InstanceId(1);

/// Renders one component once per value in `watched`, and says how many
/// times its effect ran.
fn runs_over_three_renders_of(watched: &[u32]) -> u32 {
    let runs = Rc::new(Cell::new(0));
    let slot = here!();
    for value in watched {
        let _scope = Scope::enter(ONE);
        let counter = Rc::clone(&runs);
        effect_at(slot, *value, move || {
            counter.set(counter.get() + 1);
        });
        end_render().expect("the same effect is reached every render");
    }
    runs.get()
}

/// The same, keeping what was opened and closed in order.
fn order_over_renders_of(watched: &[u32]) -> Vec<String> {
    let log = Rc::new(RefCell::new(Vec::new()));
    let slot = here!();
    for value in watched {
        let _scope = Scope::enter(ONE);
        let log = Rc::clone(&log);
        let id = *value;
        effect_at(slot, id, move || {
            log.borrow_mut().push(format!("open {id}"));
            Camera { id, log }
        });
        end_render().expect("the same effect is reached every render");
    }
    // What is still open at the end is not part of the order being
    // checked; the last `close` would say nothing about ordering.
    let order = log.borrow().clone();
    drop_instance(ONE);
    order
}

#[test]
fn an_effect_runs_once_while_what_it_watches_does_not_change() {
    assert_eq!(runs_over_three_renders_of(&[1, 1, 1]), 1);
}

#[test]
fn an_effect_runs_again_when_what_it_watches_changes() {
    assert_eq!(runs_over_three_renders_of(&[1, 2, 2]), 2);
}

#[test]
fn the_previous_guard_is_dropped_before_the_effect_runs_again() {
    // Order matters: a camera has to be closed before the next is opened.
    assert_eq!(
        order_over_renders_of(&[1, 2]),
        ["open 1", "close 1", "open 2"]
    );
}

#[test]
fn two_changes_in_one_frame_run_the_effect_once() {
    // Review Focus 4. The render is what runs effects, and a frame
    // renders once however many times the state moved.
    assert_eq!(runs_for_two_mutations_in_one_frame(), 1);
}

#[test]
fn an_effects_guard_is_dropped_when_the_component_goes() {
    assert_eq!(closed_after_unmount(), 1);
}

/// A component whose state is mutated twice before the frame that draws
/// it. The frame renders once, so the effect runs once.
fn runs_for_two_mutations_in_one_frame() -> u32 {
    let runs = Rc::new(Cell::new(0));
    let state_slot = here!();
    let effect_slot = here!();

    let render = {
        let runs = Rc::clone(&runs);
        move || {
            let _scope = Scope::enter(ONE);
            let state = cell(state_slot, || 0_u32);
            let counter = Rc::clone(&runs);
            effect_at(effect_slot, state.get(), move || {
                counter.set(counter.get() + 1);
            });
            end_render().expect("the same effect is reached every render");
            state
        }
    };

    let state = render();
    // The first render is what put the effect there; what is being
    // counted is what the next frame does.
    runs.set(0);

    state.set(1);
    state.set(2);

    let dirty = take_dirty();
    assert_eq!(dirty, vec![ONE], "one frame renders a component once");
    for _ in dirty {
        render();
    }

    runs.get()
}

/// How many cameras were closed by the component leaving the tree.
fn closed_after_unmount() -> usize {
    let log = Rc::new(RefCell::new(Vec::new()));
    {
        let _scope = Scope::enter(ONE);
        let log = Rc::clone(&log);
        effect_at(here!(), 1_u32, move || Camera { id: 1, log });
        end_render().expect("the same effect is reached every render");
    }
    assert!(log.borrow().is_empty(), "nothing is closed by a render");

    drop_instance(ONE);
    log.borrow().len()
}
