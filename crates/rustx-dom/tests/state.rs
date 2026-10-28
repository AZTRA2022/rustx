//! State cells: identified by where they are written, owned by an instance.

use rustx_dom::state::{InstanceId, Scope, Slot, cell, drop_instance, take_dirty};

/// A slot built where it is called, the way the generated code builds one.
macro_rules! here {
    () => {
        Slot {
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

struct Noisy(std::rc::Rc<std::cell::Cell<bool>>);

impl Drop for Noisy {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

#[test]
fn a_cell_keeps_its_value_across_renders_of_the_same_instance() {
    let id = InstanceId(1);
    let slot = here!();
    {
        let _scope = Scope::enter(id);
        cell(slot, || 0_i32).set(7);
    }
    let _scope = Scope::enter(id);
    assert_eq!(cell(slot, || 0_i32).get(), 7);
}

#[test]
fn two_instances_of_the_same_component_do_not_share_a_cell() {
    // Review Focus 1: the slot is the same line of code in both.
    let slot = here!();
    {
        let _scope = Scope::enter(InstanceId(2));
        cell(slot, || 0_i32).set(7);
    }
    let _scope = Scope::enter(InstanceId(3));
    assert_eq!(cell(slot, || 0_i32).get(), 0);
}

#[test]
fn mutating_a_cell_marks_its_instance_dirty_once() {
    let id = InstanceId(4);
    let _scope = Scope::enter(id);
    let n = cell(here!(), || 0_i32);
    n.set(1);
    n.update(|v| *v += 1);
    assert_eq!(take_dirty(), vec![id]);
    assert!(take_dirty().is_empty(), "dirt is consumed, not remembered");
}

#[test]
fn a_handle_cloned_into_a_closure_still_reaches_the_same_cell() {
    // A handler outlives the render that built it, so the rewrite hands
    // it a clone. The clone must not be a copy of the value.
    let _scope = Scope::enter(InstanceId(5));
    let n = cell(here!(), || 0_i32);
    let held = n.clone();
    held.set(9);
    assert_eq!(n.get(), 9);
}

#[test]
fn dropping_an_instance_runs_the_drop_of_what_it_held() {
    // Review Focus 5, at the level where it is testable without a DOM.
    let witness = std::rc::Rc::new(std::cell::Cell::new(false));
    {
        let _scope = Scope::enter(InstanceId(6));
        cell(here!(), || Noisy(witness.clone()));
    }
    drop_instance(InstanceId(6));
    assert!(witness.get(), "the value's Drop never ran");
}

#[test]
fn two_cells_on_different_lines_of_one_component_stay_apart() {
    let _scope = Scope::enter(InstanceId(7));
    let a = cell(here!(), || 1_i32);
    let b = cell(here!(), || 2_i32);
    a.set(10);
    assert_eq!(b.get(), 2);
}

#[test]
fn the_same_declaration_under_two_keys_is_two_cells() {
    // §7.4 bis: a cell identified by its place in the code is not enough
    // for n rows built from one `Vec`. Without the key, every row of a
    // list shares one piece of state.
    let _scope = Scope::enter(InstanceId(8));
    let slot = here!();

    rustx_dom::state::with_key("a", || cell(slot, || 0_i32).set(1));
    rustx_dom::state::with_key("b", || cell(slot, || 0_i32).set(2));

    assert_eq!(rustx_dom::state::with_key("a", || cell(slot, || 0_i32).get()), 1);
    assert_eq!(rustx_dom::state::with_key("b", || cell(slot, || 0_i32).get()), 2);
}

#[test]
fn a_key_follows_the_row_when_the_list_is_reordered() {
    let _scope = Scope::enter(InstanceId(9));
    let slot = here!();

    for key in ["a", "b", "c"] {
        rustx_dom::state::with_key(key, || cell(slot, || 0_i32));
    }
    rustx_dom::state::with_key("c", || cell(slot, || 0_i32).set(7));

    // The list is now c, a, b — the key is what is asked for, not the
    // position, so the value has to come back with it.
    let order = ["c", "a", "b"];
    let values: Vec<i32> = order
        .iter()
        .map(|key| rustx_dom::state::with_key(key, || cell(slot, || 0_i32).get()))
        .collect();
    assert_eq!(values, [7, 0, 0]);
}
