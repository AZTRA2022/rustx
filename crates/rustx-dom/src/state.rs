//! The cells a promoted local becomes.
//!
//! The design document asks for `let mut n = 0;` and nothing else. The
//! compiler turns that one line into a call to [`cell`], and this module
//! is what makes the value survive the next render.
//!
//! # What identifies a cell
//!
//! Two things, and both are needed. The **slot** — the file, line and
//! column the declaration was written at — tells one `let mut` from
//! another inside the same component. The **instance** tells two
//! mountings of the same component apart: `<Counter /><Counter />` is one
//! line of code and two counters, and if the slot alone identified the
//! cell they would share it.
//!
//! # Why none of this is `Send`
//!
//! State belongs to the render process's UI thread. A handler written by
//! the developer captures whatever it likes — an `Rc`, a `Cell`, a file
//! handle — and never has to argue with the borrow checker about a thread
//! it will never reach. A worker thread reaches this state the only way
//! there is: by posting a job that runs here.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// One instance's cells, by the declaration they came from.
type Slots = HashMap<(Slot, String), Rc<dyn Any>>;

/// Where a `let mut` was written. Built by the generated code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Slot {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

/// One mounted component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceId(pub u64);

thread_local! {
    /// Every live instance's cells. Dropping an entry drops its values,
    /// which is how a component's `Drop` runs at unmount.
    static CELLS: RefCell<HashMap<InstanceId, Slots>> =
        RefCell::new(HashMap::new());
    /// The instance being rendered. A stack, because a component renders
    /// inside its parent's render.
    static RENDERING: RefCell<Vec<InstanceId>> = const { RefCell::new(Vec::new()) };
    /// Instances whose state changed since the last frame.
    static DIRTY: RefCell<Vec<InstanceId>> = const { RefCell::new(Vec::new()) };
    /// The keys of the rows being rendered, outermost first.
    static KEYS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Makes `id` the instance being rendered until this guard is dropped.
///
/// A guard rather than a pair of calls, because a component's body can
/// panic and the stack still has to unwind correctly — otherwise every
/// later render would write into the wrong instance.
pub struct Scope {
    _private: (),
}

impl Scope {
    pub fn enter(id: InstanceId) -> Self {
        RENDERING.with(|stack| stack.borrow_mut().push(id));
        Self { _private: () }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        RENDERING.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

/// The instance currently rendering.
pub fn current() -> Option<InstanceId> {
    RENDERING.with(|stack| stack.borrow().last().copied())
}

/// A handle on one piece of a component's state.
///
/// Cloning is cheap and shares the cell: that is what lets a handler keep
/// hold of state after the render that built it has returned.
pub struct State<T> {
    value: Rc<RefCell<T>>,
    owner: InstanceId,
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
            owner: self.owner,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for State<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "State({:?})", self.value.borrow())
    }
}

impl<T: Clone> State<T> {
    /// The value, as a copy. Reading never marks anything dirty.
    pub fn get(&self) -> T {
        self.value.borrow().clone()
    }
}

impl<T> State<T> {
    /// Replace the value and mark this component for the next frame.
    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;
        self.soil();
    }

    /// Change the value in place — the shape `n += 1` is rewritten into.
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        change(&mut self.value.borrow_mut());
        self.soil();
    }

    /// Read without copying, for a value that is not `Clone`.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        read(&self.value.borrow())
    }

    /// Mutating does not draw. It says "this one, next frame".
    fn soil(&self) {
        DIRTY.with(|dirty| {
            let mut dirty = dirty.borrow_mut();
            if !dirty.contains(&self.owner) {
                dirty.push(self.owner);
            }
        });
    }
}

/// The cell at `slot` for the instance being rendered, created on the
/// first render and found again on every one after.
///
/// # Panics
///
/// Outside a [`Scope`]. That means generated code called a component
/// function without mounting it, which is a bug in the framework rather
/// than in anything the developer wrote — so it says so.
pub fn cell<T: 'static>(slot: Slot, initial: impl FnOnce() -> T) -> State<T> {
    let owner = current().unwrap_or_else(|| {
        panic!(
            "rustx: state was asked for at {}:{}:{} outside a render. \
             A component function has to be called through the renderer.",
            slot.file, slot.line, slot.column
        )
    });

    let value = CELLS.with(|cells| {
        let mut cells = cells.borrow_mut();
        let slots = cells.entry(owner).or_default();
        let existing = slots
            .entry((slot, key_path()))
            .or_insert_with(|| Rc::new(RefCell::new(initial())));
        Rc::clone(existing)
    });

    // The downcast cannot fail: the same slot is the same `let mut`, and
    // a `let mut` has one type.
    let value = value
        .downcast::<RefCell<T>>()
        .expect("a slot holds one type, because it is one declaration");

    State { value, owner }
}

/// The instances that changed since the last call, in the order they
/// first changed. Taking them clears them.
pub fn take_dirty() -> Vec<InstanceId> {
    DIRTY.with(|dirty| std::mem::take(&mut *dirty.borrow_mut()))
}

/// Release everything an instance held.
///
/// This is where a component's cleanup happens, and it is Rust's own:
/// dropping the cells drops the values, which runs their `Drop`. No
/// second cleanup mechanism is invented on top.
pub fn drop_instance(id: InstanceId) {
    let held = CELLS.with(|cells| cells.borrow_mut().remove(&id));
    drop(held);
    DIRTY.with(|dirty| dirty.borrow_mut().retain(|dirty| *dirty != id));
}

/// How many instances hold state. For tests and diagnostics.
pub fn live_instances() -> usize {
    CELLS.with(|cells| cells.borrow().len())
}

/// Render `body` as the row identified by `key`.
///
/// A cell is identified by where it is written, which tells one `let
/// mut` from another. That is not enough for a list: ten rows built from
/// one `Vec` are one `let mut`, written once, and without a key they
/// would share one cell — so what was typed in the third field would
/// appear in the first the moment the list reordered.
///
/// The key is the row's identity, the same one the reconciler matches
/// children by. State follows it rather than following the position.
pub fn with_key<R>(key: impl std::fmt::Display, body: impl FnOnce() -> R) -> R {
    KEYS.with(|keys| keys.borrow_mut().push(key.to_string()));
    let outcome = body();
    KEYS.with(|keys| {
        keys.borrow_mut().pop();
    });
    outcome
}

/// The keys in force, as one string. Empty outside any list.
fn key_path() -> String {
    KEYS.with(|keys| keys.borrow().join("\u{1f}"))
}
