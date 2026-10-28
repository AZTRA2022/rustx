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

    let key = (slot, key_path());
    touch(key.clone());

    let value = CELLS.with(|cells| {
        let mut cells = cells.borrow_mut();
        let slots = cells.entry(owner).or_default();
        let existing = slots
            .entry(key)
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
    REACHED_BEFORE.with(|before| before.borrow_mut().remove(&id));
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
    // A guard, for the same reason `Scope` is one: a component body can
    // panic, panics are caught at the frame boundary, and a key left on
    // the stack would silently detach every later cell on this thread
    // from the row it belongs to.
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            KEYS.with(|keys| {
                keys.borrow_mut().pop();
            });
        }
    }

    KEYS.with(|keys| keys.borrow_mut().push(key.to_string()));
    let _pop = Pop;
    body()
}

/// Nested keys are joined with a character nobody writes in a key.
const KEY_SEPARATOR: char = '\x1f';

/// The keys in force, as one string. Empty outside any list.
fn key_path() -> String {
    KEYS.with(|keys| keys.borrow().join("\u{1f}"))
}

/// A value built once and held for as long as its component lives.
///
/// The same cell a `let mut` becomes, minus the writing. A camera, a
/// file, a background task's cancellation token: anything a component
/// opens and should close again. There is no closing to write, because
/// the value is dropped when the component is, and `Drop` is Rust's own
/// cleanup — the framework does not invent a second one.
pub struct Kept<T> {
    value: Rc<T>,
}

impl<T> Clone for Kept<T> {
    fn clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
        }
    }
}

impl<T> Kept<T> {
    /// Read without copying, for a value that is not `Clone` — which a
    /// camera or a file handle will not be.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        read(&self.value)
    }
}

impl<T: Clone> Kept<T> {
    pub fn get(&self) -> T {
        (*self.value).clone()
    }
}

/// Build `value` on the first render of this component and hand back the
/// same one on every render after.
pub fn keep_at<T: 'static>(slot: Slot, build: impl FnOnce() -> T) -> Kept<T> {
    let owner = current().unwrap_or_else(|| {
        panic!(
            "rustx: a kept value was asked for at {}:{}:{} outside a render.",
            slot.file, slot.line, slot.column
        )
    });
    let key = (slot, key_path());
    touch(key.clone());

    let value = CELLS.with(|cells| {
        let mut cells = cells.borrow_mut();
        let slots = cells.entry(owner).or_default();
        let existing = slots
            .entry(key)
            .or_insert_with(|| Rc::new(build()) as Rc<dyn Any>);
        Rc::clone(existing)
    });

    let value = value
        .downcast::<T>()
        .expect("a slot holds one type, because it is one call");
    Kept { value }
}

thread_local! {
    /// Slots reached during the render in progress.
    static TOUCHED: RefCell<Vec<(Slot, String)>> = const { RefCell::new(Vec::new()) };
    /// What the previous render of each instance reached.
    static REACHED_BEFORE: RefCell<HashMap<InstanceId, Vec<(Slot, String)>>> =
        RefCell::new(HashMap::new());
}

fn touch(key: (Slot, String)) {
    TOUCHED.with(|touched| touched.borrow_mut().push(key));
}

/// Close a render, and say so if it stopped reaching something it used
/// to reach.
///
/// Identity by place assumes the same lines run every time. A `keep`
/// inside an `if` breaks that assumption quietly: the value comes back
/// attached to a line that did not ask for it this time, and the bug
/// surfaces much later as a stale camera or a count that will not
/// reset. Saying so here costs one comparison per render and turns a
/// mystery into a sentence.
///
/// Called by the engine at the end of each render; public so that it
/// can be exercised without one.
pub fn end_render() -> Result<(), String> {
    let Some(instance) = current() else {
        return Ok(());
    };
    let reached = TOUCHED.with(|touched| std::mem::take(&mut *touched.borrow_mut()));

    // A row that left a keyed list takes its cells with it. That has to
    // happen before the check below, or a row the developer legitimately
    // removed would be reported as a value this render forgot to reach.
    let gone: Vec<String> = REACHED_BEFORE.with(|before| {
        let before = before.borrow();
        let Some(previous) = before.get(&instance) else {
            return Vec::new();
        };
        let mut gone: Vec<String> = previous
            .iter()
            .filter(|key| !reached.contains(key))
            .filter_map(|(_, path)| path.split(KEY_SEPARATOR).next())
            .filter(|row| !row.is_empty())
            .filter(|row| !reached.iter().any(|(_, path)| path.starts_with(*row)))
            .map(str::to_string)
            .collect();
        gone.sort();
        gone.dedup();
        gone
    });
    for row in gone {
        drop_key(instance, &row);
    }

    // Two calls landing on one cell means two components are sharing a
    // piece of state that looks private. Saying so is the difference
    // between a puzzle and a sentence.
    let shared = reached.iter().enumerate().find_map(|(at, key)| {
        reached[..at]
            .iter()
            .any(|earlier| earlier == key)
            .then_some(key)
    });
    if let Some((slot, _)) = shared {
        let complaint = format!(
            "rustx: the state at {}:{}:{} was asked for twice in one render. \n             Two components are sharing it — wrap each call in `with_key` so \n             they get one each.",
            slot.file, slot.line, slot.column
        );
        REACHED_BEFORE.with(|before| before.borrow_mut().insert(instance, reached));
        return Err(complaint);
    }

    let complaint = REACHED_BEFORE.with(|before| {
        let before = before.borrow();
        let previous = before.get(&instance)?;
        previous
            .iter()
            .find(|key| !reached.contains(key))
            .map(|(slot, _)| {
                format!(
                    "rustx: the value kept at {}:{}:{} was not reached by this render. \n                     A kept value has to be asked for on every render — move it out of \n                     the `if`, or give the branch a key with `with_key`.",
                    slot.file, slot.line, slot.column
                )
            })
    });

    REACHED_BEFORE.with(|before| before.borrow_mut().insert(instance, reached));
    match complaint {
        Some(complaint) => Err(complaint),
        None => Ok(()),
    }
}

/// Release the cells a keyed row held.
///
/// A cell is identified by instance, place and key. `drop_instance`
/// removes a whole instance, but a row leaving a list is not a whole
/// instance: without this, a list's state map grows for every row that
/// ever existed, a row that comes back resurrects its old value, and a
/// row holding a file handle never runs its `Drop`.
pub fn drop_key(instance: InstanceId, key: &str) {
    let prefix = key.to_string();
    let held: Vec<Rc<dyn Any>> = CELLS.with(|cells| {
        let mut cells = cells.borrow_mut();
        let Some(slots) = cells.get_mut(&instance) else {
            return Vec::new();
        };
        let going: Vec<(Slot, String)> = slots
            .keys()
            .filter(|(_, path)| path == &prefix || path.starts_with(&format!("{prefix}\u{1f}")))
            .cloned()
            .collect();
        going.iter().filter_map(|key| slots.remove(key)).collect()
    });
    drop(held);

    REACHED_BEFORE.with(|before| {
        if let Some(reached) = before.borrow_mut().get_mut(&instance) {
            reached.retain(|(_, path)| {
                path != &prefix && !path.starts_with(&format!("{prefix}\u{1f}"))
            });
        }
    });
}

/// The keys this render has reached, so the engine can tell which rows
/// of a keyed list have gone.
pub fn reached_keys() -> Vec<String> {
    TOUCHED.with(|touched| {
        let mut keys: Vec<String> = touched
            .borrow()
            .iter()
            .filter(|(_, path)| !path.is_empty())
            .map(|(_, path)| path.split('\u{1f}').next().unwrap_or_default().to_string())
            .collect();
        keys.sort();
        keys.dedup();
        keys
    })
}
