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
use std::sync::{Arc, RwLock};

use crate::state::{Cancel, InstanceId, Remote, current, forget_on_drop, mark_dirty};

/// Who is showing a store's value. Shared with the registry, so that an
/// unmounted component can be dropped from the list without knowing
/// what the store holds.
type Readers = Rc<RefCell<Vec<InstanceId>>>;

/// The value, and who is showing it.
///
/// The value sits behind a lock rather than a `RefCell`, and in an `Arc`
/// rather than an `Rc`, so that a future can read it where it is
/// instead of being handed a copy somebody has to keep up to date. That
/// is the whole of what `let mut clicks = 0;` at the top of a page used
/// to need an `Arc<Mutex<…>>` beside it for.
///
/// What it costs: an atomic counter and an uncontended lock, on a
/// handful of values per application read a few times per frame. What
/// it does not cost is a clone on any write, which is the thing a page
/// would have felt. A component's own cell is untouched and stays an
/// `Rc<RefCell<T>>` — there are thousands of those, and they are the
/// hot path.
struct Held<T> {
    value: Arc<RwLock<T>>,
    readers: Readers,
}

impl<T> Held<T> {
    /// The value, for as long as `with` runs.
    ///
    /// A poisoned lock is read through: the panic that poisoned it was
    /// somebody else's, and a page that refuses to draw because of it
    /// is a worse answer than a page that draws what is there.
    fn read<R>(&self, with: impl FnOnce(&T) -> R) -> R {
        with(&self.value.read().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    /// The value, to change in place.
    fn write<R>(&self, with: impl FnOnce(&mut T) -> R) -> R {
        with(&mut self.value.write().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    /// The value, if reading it right now would not wait.
    fn read_if_free<R>(&self, with: impl FnOnce(&T) -> R) -> Option<R> {
        self.value.try_read().ok().map(|held| with(&held))
    }
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
        shared.read(|held| held.clone())
    }
}

impl<T: Default + 'static> Store<T> {
    /// Read without copying, for a value that is not `Clone`.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        let shared = held(Key::Type(TypeId::of::<T>()), T::default);
        enrol(&shared);
        shared.read(read)
    }

    /// Replace the value and mark everyone who reads it.
    ///
    /// The value that was there is dropped once the lock is released: a
    /// `Drop` is the developer's code, and it may well read this store.
    pub fn set(&self, value: T) {
        let shared = held(Key::Type(TypeId::of::<T>()), T::default);
        let previous = shared.write(|held| std::mem::replace(held, value));
        drop(previous);
        soil(&shared);
    }

    /// Change the value in place.
    ///
    /// `change` runs while the store is held, so it must not read the
    /// same store — the way a `RefCell` has always behaved, and the way
    /// a lock does too.
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        let shared = held(Key::Type(TypeId::of::<T>()), T::default);
        shared.write(change);
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
    // What `rustx dev` carried over from the process it replaced, if it
    // carried anything, and if what it carried is still this kind of
    // value. Outside `rustx dev` there is nothing to ask, and the ask
    // costs a lookup in an empty table.
    let held = held(Key::Name(name), || {
        crate::carried::remembered::<T>(name).unwrap_or_else(initial)
    });

    // And the other direction: how to ask this value what it says, so
    // that it can be carried over in turn. The handle is weak, because a
    // reporter that held the value would be the reason it never dies.
    let watching = Rc::downgrade(&held);
    crate::carried::report::<T>(name, move || {
        let held = watching.upgrade()?;
        held.read_if_free(|value| crate::carried::text_of::<T>(value))
            .flatten()
    });

    Shared { held, name }
}

/// A value a page declared, reached from a future.
///
/// `Send`, `Sync` and `Clone`, and it carries the value rather than a
/// copy of it: a read goes to where the value is. That is what lets a
/// bare `async { … }` write `format!("clics: {clicks}")` with nothing
/// declared beside it.
///
/// Reading and writing are deliberately not symmetric. A read is direct
/// because a future asking what the page says has nothing to wait for;
/// a write posts, because marking the page dirty, coalescing, and
/// refusing a departed component's write all happen on the thread that
/// draws. See [`Shared::reach_until`].
pub struct Reach<T> {
    value: Arc<RwLock<T>>,
    posting: Remote<T>,
}

impl<T> Clone for Reach<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
            posting: self.posting.clone(),
        }
    }
}

impl<T: 'static> Reach<T> {
    /// The value, for as long as `read` runs.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        read(&self.value.read().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    /// Change the value, on the next frame of the thread that owns it.
    pub fn update(&self, change: impl FnOnce(&mut T) + Send + 'static) {
        self.posting.update(change);
    }

    /// Replace the value, on the next frame of the thread that owns it.
    pub fn set(&self, value: T)
    where
        T: Send,
    {
        self.posting.set(value);
    }

    /// Is there still somewhere for a write through this handle to go?
    pub fn is_open(&self) -> bool {
        self.posting.is_open()
    }
}

impl<T: Clone + 'static> Reach<T> {
    /// The value, as a copy — the shape a page's own read takes.
    pub fn get(&self) -> T {
        self.with(|held| held.clone())
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
    /// The name it was declared under. Carried so that a handle which
    /// crosses a thread can say which value it means without holding it.
    name: &'static str,
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            held: Rc::clone(&self.held),
            name: self.name,
        }
    }
}

impl<T: Clone + 'static> Shared<T> {
    /// The value, as a copy.
    pub fn get(&self) -> T {
        enrol(&self.held);
        self.held.read(|held| held.clone())
    }
}

impl<T: 'static> Shared<T> {
    /// Read without copying, for a value that is not `Clone`.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        enrol(&self.held);
        self.held.read(read)
    }

    /// Replace the value and mark everyone who reads it.
    ///
    /// What was there is dropped once the lock is released: a `Drop` is
    /// the developer's code, and it may well read this value.
    pub fn set(&self, value: T) {
        let previous = self.held.write(|held| std::mem::replace(held, value));
        drop(previous);
        soil(&self.held);
        crate::carried::keep();
    }

    /// A handle on this value that may cross a thread.
    ///
    /// What a `task(async { … })` captures when the page declared the
    /// value at its top. Writing through it posts on the mailbox of the
    /// thread this call is made on, and the frame applies it.
    pub fn remote(&self) -> crate::state::Remote<T> {
        crate::state::remote_for_shared(self.name, None)
    }

    /// The same, bound to a task: when `until` is cancelled, what this
    /// handle had already posted is refused instead of applied. A value
    /// found by a name outlives every page, so this is the only thing
    /// that stops a departed component's future writing into the next
    /// one.
    pub fn remote_until(&self, until: &crate::state::Cancel) -> crate::state::Remote<T> {
        crate::state::remote_for_shared(self.name, Some(until.clone()))
    }

    /// A handle a future can **read** as well as write.
    ///
    /// What a bare `async { … }` captures when the block reads the
    /// value and not only writes it. Reading goes straight to where the
    /// value is — no copy is published and no clone is added to a
    /// write — and writing still posts on the mailbox, so the frame
    /// keeps marking the page, coalescing a thousand writes into one
    /// render, and refusing a write whose component has gone.
    ///
    /// The consequence to know: a future that writes and then reads
    /// sees the old value until the frame applies the write.
    pub fn reach_until(&self, until: &Cancel) -> Reach<T>
    where
        T: Send + Sync,
    {
        Reach {
            value: Arc::clone(&self.held.value),
            posting: crate::state::remote_for_shared(self.name, Some(until.clone())),
        }
    }

    /// Change the value in place.
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        self.held.write(change);
        soil(&self.held);
        // Written as it happens: the window `rustx dev` replaces is
        // killed, so nothing that waits for a tidy shutdown would run.
        // A no-op when nobody asked for a file, which is everywhere but
        // `rustx dev`.
        crate::carried::keep();
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
                    value: Arc::new(RwLock::new(initial())),
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

/// Apply `change` to the value declared under `name`, or say it has gone.
///
/// Runs on the thread that owns the stores, which is the only thread
/// that can look in them. A value nobody has asked for yet has nothing
/// to change: a write that arrives first is thrown away rather than
/// creating the value from an initializer this function does not have.
pub(crate) fn write_to_shared<T: 'static>(name: &'static str, change: impl FnOnce(&mut T)) -> bool {
    let found = STORES.with(|stores| {
        let stores = stores.borrow();
        stores
            .get(&Key::Name(name))
            .map(|entry| Rc::clone(&entry.held))
    });
    let Some(found) = found else {
        return false;
    };
    let Ok(held) = found.downcast::<Held<T>>() else {
        return false;
    };
    held.write(change);
    soil(&held);
    // Not `carried::keep()` here: that is a synchronous write of the
    // whole carried-state file, and a thousand writes from an `async`
    // loop are a thousand jobs in one frame. The frame persists once,
    // after it has drained them all.
    crate::state::carried_is_stale();
    true
}
