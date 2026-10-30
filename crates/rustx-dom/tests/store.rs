//! Data two distant components share without a parent handing it down.

use rustx_dom::state::{InstanceId, Scope, drop_instance, take_dirty};
use rustx_dom::store::{reset, store};

/// What a component does when it shows what the store holds.
fn read_from(id: InstanceId) -> u32 {
    let _scope = Scope::enter(id);
    store::<u32>().get()
}

#[test]
fn two_components_asking_for_the_same_type_get_the_same_store() {
    let first = Scope::enter(InstanceId(1));
    store::<u32>().set(5);
    drop(first);

    let _second = Scope::enter(InstanceId(2));
    assert_eq!(store::<u32>().get(), 5);
}

#[test]
fn mutating_a_store_marks_everyone_who_read_it() {
    // Review Focus 3, the first half.
    read_from(InstanceId(1));
    read_from(InstanceId(2));
    store::<u32>().set(9);

    let mut dirty = take_dirty();
    dirty.sort();
    assert_eq!(dirty, vec![InstanceId(1), InstanceId(2)]);
}

#[test]
fn a_component_that_never_read_the_store_is_left_alone() {
    read_from(InstanceId(1));
    let _second = Scope::enter(InstanceId(2)); // renders, reads nothing
    store::<u32>().set(9);

    assert_eq!(take_dirty(), vec![InstanceId(1)]);
}

#[test]
fn an_unmounted_reader_is_forgotten_rather_than_kept_alive() {
    // Review Focus 3, the second half: a store must not be a leak of
    // every component that ever looked at it.
    read_from(InstanceId(1));
    read_from(InstanceId(2));
    drop_instance(InstanceId(2));
    store::<u32>().set(9);

    assert_eq!(take_dirty(), vec![InstanceId(1)]);
}

#[test]
fn a_read_outside_a_render_enrols_nobody() {
    // A worker thread's posted job reads the store on the UI thread
    // while nothing is rendering. There is no component to mark.
    assert_eq!(store::<u32>().get(), 0);
    store::<u32>().set(3);

    assert!(take_dirty().is_empty(), "a read outside a render enrolled someone");
}

#[test]
fn a_reader_is_enrolled_once_however_often_it_reads() {
    let _scope = Scope::enter(InstanceId(1));
    for _ in 0..3 {
        store::<u32>().get();
    }
    drop(_scope);
    store::<u32>().set(1);

    assert_eq!(take_dirty(), vec![InstanceId(1)]);
}

#[test]
fn a_store_that_is_not_clone_is_read_without_copying() {
    let _scope = Scope::enter(InstanceId(1));
    store::<Vec<u8>>().update(|held| held.push(7));

    assert_eq!(store::<Vec<u8>>().with(Vec::len), 1);
}

#[test]
fn a_reset_store_starts_again_from_its_default() {
    store::<u32>().set(42);
    reset();

    assert_eq!(store::<u32>().get(), 0);
}

#[test]
fn a_handle_taken_before_a_reset_sees_the_store_that_came_after() {
    // A handle kept by a component, or captured in a handler, must not
    // go on writing into a store nobody else can read: a write nobody
    // sees is worse than a panic.
    let held = store::<u32>();
    held.set(5);
    reset();

    held.set(9);
    assert_eq!(store::<u32>().get(), 9, "the handle wrote into an orphan");
}

// ---------------------------------------------------------------------
// A value declared at the top of a page: shared by the application,
// built from what was written rather than from `Default`.
// ---------------------------------------------------------------------

use rustx_dom::store::shared;

/// What the compiler makes per declaration: a type that names the
/// value and holds nothing.
#[allow(non_camel_case_types)]
struct points;
#[allow(non_camel_case_types)]
struct also_points;

#[test]
fn a_shared_value_is_built_once_and_found_again() {
    rustx_dom::store::reset();
    let mut built = 0;
    let first = shared::<points, _>(|| {
        built += 1;
        7_u32
    });
    assert_eq!(first.get(), 7);

    let again = shared::<points, _>(|| {
        built += 1;
        9_u32
    });
    assert_eq!(again.get(), 7, "the second ask found the first value");
    assert_eq!(built, 1, "the initializer runs once");
}

#[test]
fn two_markers_are_two_values_even_of_the_same_type() {
    rustx_dom::store::reset();
    shared::<points, _>(|| 1_u32).set(10);
    shared::<also_points, _>(|| 1_u32).set(20);
    assert_eq!(shared::<points, _>(|| 1_u32).get(), 10);
    assert_eq!(shared::<also_points, _>(|| 1_u32).get(), 20);
}

#[test]
fn writing_a_shared_value_marks_everyone_who_read_it() {
    use rustx_dom::state::{Scope, next_instance, take_dirty};

    rustx_dom::store::reset();
    let _ = take_dirty();

    let reader = next_instance();
    {
        let _scope = Scope::enter(reader);
        let _ = shared::<points, _>(|| 0_u32).get();
    }
    shared::<points, _>(|| 0_u32).set(1);
    assert!(take_dirty().contains(&reader));
}

#[test]
fn a_store_is_a_shared_value_whose_marker_is_its_type() {
    // The store of plan 4 is the same mechanism, keyed by the type it
    // holds. Nothing about it changes.
    #[derive(Default, Clone, PartialEq, Debug)]
    struct Selection(String);

    rustx_dom::store::reset();
    rustx_dom::store::store::<Selection>().set(Selection("a".into()));
    assert_eq!(
        shared::<Selection, _>(Selection::default).get(),
        Selection("a".into())
    );
}
