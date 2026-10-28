//! A value built once, for as long as its component lives.

use std::cell::Cell;
use std::rc::Rc;

use rustx_dom::state::{InstanceId, Scope, Slot, drop_instance, end_render, keep_at};

macro_rules! here {
    () => {
        Slot {
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

struct Noisy(Rc<Cell<bool>>);

impl Drop for Noisy {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

#[test]
fn a_kept_value_is_built_once_however_many_renders() {
    let built = Rc::new(Cell::new(0));
    let slot = here!();
    for _ in 0..3 {
        let _scope = Scope::enter(InstanceId(1));
        let counter = built.clone();
        keep_at(slot, move || {
            counter.set(counter.get() + 1);
            7_i32
        });
    }
    assert_eq!(built.get(), 1);
}

#[test]
fn a_kept_value_is_dropped_when_its_component_goes() {
    let witness = Rc::new(Cell::new(false));
    {
        let _scope = Scope::enter(InstanceId(2));
        keep_at(here!(), || Noisy(witness.clone()));
    }
    drop_instance(InstanceId(2));
    assert!(witness.get(), "the kept value's Drop never ran");
}

#[test]
fn a_kept_value_need_not_be_clone() {
    let _scope = Scope::enter(InstanceId(3));
    let kept = keep_at(here!(), || std::cell::RefCell::new(vec![1_u8]));
    assert_eq!(kept.with(|value| value.borrow().len()), 1);
}

#[test]
fn a_keep_that_stops_running_every_render_is_reported() {
    // Review Focus 5: identity by place assumes the same code runs each
    // time. A `keep` behind an `if` does not, and what a later render
    // gets back is not what that line appears to ask for.
    let first = here!();
    let second = here!();
    {
        let _scope = Scope::enter(InstanceId(4));
        keep_at(first, || 1_i32);
        keep_at(second, || 2_i32);
        end_render().expect("a first render cannot be inconsistent");
    }

    let _scope = Scope::enter(InstanceId(4));
    keep_at(first, || 1_i32);
    let complaint = end_render().expect_err("the skipped keep should be reported");
    assert!(complaint.contains("every render"), "got {complaint}");
    assert!(complaint.contains("keep.rs"), "the file is not named: {complaint}");
}

#[test]
fn a_render_that_keeps_reaching_everything_is_quiet() {
    let slot = here!();
    for _ in 0..3 {
        let _scope = Scope::enter(InstanceId(5));
        keep_at(slot, || 1_i32);
        end_render().expect("nothing changed");
    }
}
