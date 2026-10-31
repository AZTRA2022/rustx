//! Data a component reads without being handed it.
//!
//! The device list and the preview panel of the proof-of-concept
//! software show the same selection, and they are nowhere near each
//! other in the tree. Threading that value from their nearest common
//! parent through every component in between is the cost React pays for
//! having no other way; a store is the other way.
//!
//! A store is a cell that belongs to nobody. It is found by its type —
//! `store::<Selection>()` is the application's one selection — created
//! on the first ask, and it keeps the list of the instances that have
//! read it so that writing to it can mark exactly those for the next
//! frame.
//!
//! # What it is not
//!
//! It is not shared between threads. State lives on the render
//! process's UI thread, and a store is no exception: it is shared
//! between *components*, not between threads. A worker thread reaches it
//! the only way there is, by posting a job that runs here.
//!
//! It is not shared between windows either. That one is the OS layer's
//! to build (design document §7.4, multi-window), because a second
//! window is what gives it a reason to exist.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::state::{InstanceId, current, forget_on_drop, mark_dirty};

/// Who is showing a store's value. Shared with the registry, so that an
/// unmounted component can be dropped from the list without knowing
/// what the store holds.
type Readers = Rc<RefCell<Vec<InstanceId>>>;

/// The value, and who is showing it.
struct Held<T> {
    value: RefCell<T>,
    readers: Readers,
}

/// A handle on one shared value.
///
/// The same shape as `State<T>`, minus the owner: nobody holds it, so
/// nothing drops it, and it outlives every component that reads it.
///
/// It holds no value of its own — only the type, which is the store's
/// name. A handle kept by a component or captured in a handler
/// therefore always reaches the store that is there *now*, and a
/// [`reset`] cannot leave it writing into an orphan nobody else can
/// read.
pub struct Store<T> {
    of: PhantomData<fn() -> T>,
}

impl<T> Clone for Store<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Store<T> {}

impl<T: Default + Clone + 'static> Store<T> {
    /// The value, as a copy. Reading during a render enrols the
    /// component as a reader, which is what makes a later write reach
    /// the screen.
    pub fn get(&self) -> T {
        let shared = held(Key::Type(TypeId::of::<T>()), T::default);
        enrol(&shared);
        shared.value.borrow().clone()
    }
}

impl<T: Default + 'static> Store<T> {
    /// Read without copying, for a value that is not `Clone`.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        let shared = held(Key::Type(TypeId::of::<T>()), T::default);
        enrol(&shared);
        read(&shared.value.borrow())
    }

    /// Replace the value and mark everyone who reads it.
    ///
    /// The value that was there is dropped once the borrow is over: a
    /// `Drop` is the developer's code, and it may well read this store.
    pub fn set(&self, value: T) {
        let shared = held(Key::Type(TypeId::of::<T>()), T::default);
        let previous = std::mem::replace(&mut *shared.value.borrow_mut(), value);
        drop(previous);
        soil(&shared);
    }

    /// Change the value in place.
    ///
    /// `change` runs while the store is borrowed, so it must not read
    /// the same store — the way a `RefCell` has always behaved.
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        let shared = held(Key::Type(TypeId::of::<T>()), T::default);
        change(&mut shared.value.borrow_mut());
        soil(&shared);
    }
}

/// Remember that the component now rendering shows this value.
///
/// Outside a render there is nobody to remember: a posted job reading
/// the store is not a component.
fn enrol<T>(shared: &Held<T>) {
    let Some(reader) = current() else {
        return;
    };
    let mut readers = shared.readers.borrow_mut();
    if !readers.contains(&reader) {
        readers.push(reader);
    }
}

/// Writing does not draw. It says "these ones, next frame".
fn soil<T>(shared: &Held<T>) {
    let readers = shared.readers.borrow().clone();
    for reader in readers {
        mark_dirty(reader);
    }
}

/// What a value is filed under.
///
/// A store is filed under the type it holds, which is its name. A
/// value written as `let mut points = 0;` is filed under `points` —
/// the name it was written with, which is what lets two pages of one
/// application mean the same value by writing the same name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Key {
    Type(TypeId),
    Name(&'static str),
}

/// One store, as the registry holds it.
struct Entry {
    /// The value itself, found again by the marker that named it.
    held: Rc<dyn Any>,
    /// The same reader list the store has, reachable without `T`.
    readers: Readers,
}

thread_local! {
    /// One store per type, for as long as this thread lives.
    static STORES: RefCell<HashMap<Key, Entry>> = RefCell::new(HashMap::new());
    /// Whether the stores are being told about unmounted components yet.
    static FORGETTING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The store of this type, created on the first ask.
///
/// `Default` is what the first ask builds it with: a store nobody has
/// written to yet is the empty list, the unselected device, the zero.
pub fn store<T: Default + 'static>() -> Store<T> {
    let _ = held(Key::Type(TypeId::of::<T>()), T::default);
    Store { of: PhantomData }
}

/// The value this name was written with, built by `initial` on the
/// first ask.
///
/// This is what a `let mut` written at the top of a page becomes. The
/// name is the one in the source, so two pages of one application that
/// write `let mut selected = …;` mean the same value — which is what a
/// list and the screen it opens need, and what a store found by its
/// type could only give them by inventing a type to hold it in.
///
/// `initial` runs once per thread, on the first ask. Every ask after
/// finds what is there, whatever it was given.
pub fn shared<T: 'static>(name: &'static str, initial: impl FnOnce() -> T) -> Shared<T> {
    Shared {
        held: held(Key::Name(name), initial),
    }
}

/// A handle on one shared value.
///
/// Reading through it during a render enrols the component, so a later
/// write reaches the screen; writing marks everyone who read it. It is
/// the same contract as a component's own cell, which is the point: a
/// `let mut` is a `let mut` wherever it is written.
pub struct Shared<T> {
    held: Rc<Held<T>>,
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            held: Rc::clone(&self.held),
        }
    }
}

impl<T: Clone + 'static> Shared<T> {
    /// The value, as a copy.
    pub fn get(&self) -> T {
        enrol(&self.held);
        self.held.value.borrow().clone()
    }
}

impl<T: 'static> Shared<T> {
    /// Read without copying, for a value that is not `Clone`.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        enrol(&self.held);
        read(&self.held.value.borrow())
    }

    /// Replace the value and mark everyone who reads it.
    ///
    /// What was there is dropped once the borrow is over: a `Drop` is
    /// the developer's code, and it may well read this value.
    pub fn set(&self, value: T) {
        let previous = std::mem::replace(&mut *self.held.value.borrow_mut(), value);
        drop(previous);
        soil(&self.held);
    }

    /// Change the value in place.
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        change(&mut self.held.value.borrow_mut());
        soil(&self.held);
    }
}

/// The value itself, created on the first ask.
fn held<T: 'static>(key: Key, initial: impl FnOnce() -> T) -> Rc<Held<T>> {
    forget_unmounted_readers();

    let named = key.clone();
    let held = STORES.with(|stores| {
        let mut stores = stores.borrow_mut();
        let existing = stores.entry(key).or_insert_with(|| {
            let readers: Readers = Rc::new(RefCell::new(Vec::new()));
            Entry {
                held: Rc::new(Held {
                    value: RefCell::new(initial()),
                    readers: Rc::clone(&readers),
                }),
                readers,
            }
        });
        Rc::clone(&existing.held)
    });

    held.downcast::<Held<T>>().unwrap_or_else(|_| {
        // Two pages meaning the same name is the point. Two pages
        // meaning the same name with two different kinds of value is a
        // mistake, and this is the only moment anything can see it.
        match named {
            Key::Name(name) => panic!(
                "rustx: two pages declare `{name}` as values of different types. \
                 The same name is the same value, so it has to be the same kind of \
                 value — rename one of them."
            ),
            Key::Type(_) => panic!("a store is found by its type, so it holds that type"),
        }
    })
}

/// A store must not be the reason a component that left the tree is
/// still referenced.
///
/// The hook is registered on the first store rather than at start-up:
/// `state` knows nothing about stores, and an application that never
/// asks for one pays nothing for them.
fn forget_unmounted_readers() {
    if FORGETTING.with(|hooked| hooked.replace(true)) {
        return;
    }
    forget_on_drop(|gone| {
        STORES.with(|stores| {
            for entry in stores.borrow().values() {
                entry.readers.borrow_mut().retain(|reader| *reader != gone);
            }
        });
    });
}

/// Empty every store on this thread. For tests, and for a context that
/// is being replaced.
pub fn reset() {
    STORES.with(|stores| stores.borrow_mut().clear());
}
