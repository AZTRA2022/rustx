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
use std::rc::Rc;

use crate::state::{InstanceId, current, forget_on_drop, mark_dirty};

/// Who is showing a store's value. Shared with the registry, so that an
/// unmounted component can be dropped from the list without knowing
/// what the store holds.
type Readers = Rc<RefCell<Vec<InstanceId>>>;

/// The value, and who is showing it.
struct Shared<T> {
    value: RefCell<T>,
    readers: Readers,
}

/// A handle on the store of one type.
///
/// The same shape as `State<T>`, minus the owner: nobody holds it, so
/// nothing drops it, and it outlives every component that reads it.
pub struct Store<T> {
    shared: Rc<Shared<T>>,
}

impl<T> Clone for Store<T> {
    fn clone(&self) -> Self {
        Self {
            shared: Rc::clone(&self.shared),
        }
    }
}

impl<T: Clone> Store<T> {
    /// The value, as a copy. Reading during a render enrols the
    /// component as a reader, which is what makes a later write reach
    /// the screen.
    pub fn get(&self) -> T {
        self.enrol();
        self.shared.value.borrow().clone()
    }
}

impl<T> Store<T> {
    /// Read without copying, for a value that is not `Clone`.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        self.enrol();
        read(&self.shared.value.borrow())
    }

    /// Replace the value and mark everyone who reads it.
    pub fn set(&self, value: T) {
        *self.shared.value.borrow_mut() = value;
        self.soil();
    }

    /// Change the value in place.
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        change(&mut self.shared.value.borrow_mut());
        self.soil();
    }

    /// Remember that the component now rendering shows this value.
    ///
    /// Outside a render there is nobody to remember: a posted job
    /// reading the store is not a component.
    fn enrol(&self) {
        let Some(reader) = current() else {
            return;
        };
        let mut readers = self.shared.readers.borrow_mut();
        if !readers.contains(&reader) {
            readers.push(reader);
        }
    }

    /// Writing does not draw. It says "these ones, next frame".
    fn soil(&self) {
        for reader in self.shared.readers.borrow().iter() {
            mark_dirty(*reader);
        }
    }
}

/// One store, as the registry holds it.
struct Entry {
    /// The `Shared<T>` itself, found again by the type it holds.
    shared: Rc<dyn Any>,
    /// The same reader list the store has, reachable without `T`.
    readers: Readers,
}

thread_local! {
    /// One store per type, for as long as this thread lives.
    static STORES: RefCell<HashMap<TypeId, Entry>> = RefCell::new(HashMap::new());
    /// Whether the stores are being told about unmounted components yet.
    static FORGETTING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The store of this type, created on the first ask.
///
/// `Default` is what the first ask builds it with: a store nobody has
/// written to yet is the empty list, the unselected device, the zero.
pub fn store<T: Default + 'static>() -> Store<T> {
    forget_unmounted_readers();

    let shared = STORES.with(|stores| {
        let mut stores = stores.borrow_mut();
        let existing = stores.entry(TypeId::of::<T>()).or_insert_with(|| {
            let readers: Readers = Rc::new(RefCell::new(Vec::new()));
            Entry {
                shared: Rc::new(Shared {
                    value: RefCell::new(T::default()),
                    readers: Rc::clone(&readers),
                }),
                readers,
            }
        });
        Rc::clone(&existing.shared)
    });

    let shared = shared
        .downcast::<Shared<T>>()
        .expect("a store is found by its type, so it holds that type");

    Store { shared }
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
                entry
                    .readers
                    .borrow_mut()
                    .retain(|reader| *reader != gone);
            }
        });
    });
}

/// Empty every store on this thread. For tests, and for a context that
/// is being replaced.
pub fn reset() {
    STORES.with(|stores| stores.borrow_mut().clear());
}
