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
        // One frame per render in progress. A component renders inside
        // its parent's render, so without a frame of its own the
        // child's cells would be counted as the parent's and the
        // reach-check of plan 4 would accuse both of forgetting the
        // other's values.
        let keys_at = KEYS.with(|keys| keys.borrow().len());
        FRAMES.with(|frames| {
            frames.borrow_mut().push(Frame {
                keys_at,
                ..Frame::default()
            })
        });
        Self { _private: () }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        RENDERING.with(|stack| {
            stack.borrow_mut().pop();
        });
        FRAMES.with(|frames| {
            frames.borrow_mut().pop();
        });
    }
}

/// What one render in progress has reached.
#[derive(Default)]
struct Frame {
    /// The cells, kept values and effects asked for.
    touched: Vec<(Slot, String)>,
    /// The components declared, in the order they were written.
    children: Vec<InstanceId>,
    /// How many keys were already in force when this render began.
    ///
    /// A component's cells are keyed by the keys *it* opened, not by the
    /// ones it was rendered under. The router renders every page inside
    /// `with_key(pattern)`, and without this baseline the page's own
    /// keyed rows all began with the pattern: a row that left the list
    /// was then looked for by the outermost key, so removing one row
    /// would have taken every value the page held. The key it was
    /// rendered under is already in its instance number — that is what
    /// `component_at` keys a child by — so it has no business being in
    /// its cells' keys as well.
    keys_at: usize,
}

thread_local! {
    /// One frame per render on the stack, innermost last.
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
    /// The next instance number, for roots and for children alike.
    static NEXT_INSTANCE: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
    /// A child's identity: the same call site under the same parent is
    /// the same component, render after render.
    static CHILD_IDS: RefCell<HashMap<(InstanceId, Slot, String), InstanceId>> =
        RefCell::new(HashMap::new());
    /// The children each instance declared on its last render.
    static CHILDREN: RefCell<HashMap<InstanceId, Vec<InstanceId>>> =
        RefCell::new(HashMap::new());
    /// Which instance renders each child.
    ///
    /// The engine re-renders roots, because a root is what it mounted
    /// and what it has a host element for. A child marked dirty — a
    /// layout whose own cell changed, a row that counts its own clicks
    /// — therefore has to be turned into the root that draws it, and
    /// this is what turns it.
    static PARENTS: RefCell<HashMap<InstanceId, InstanceId>> = RefCell::new(HashMap::new());
}

/// An instance number nobody else has.
///
/// Roots and children draw from one counter: the engine mounts roots and
/// this module makes children, and two numbering schemes would collide
/// the moment an application had both.
pub fn next_instance() -> InstanceId {
    InstanceId(NEXT_INSTANCE.with(|next| {
        let id = next.get();
        next.set(id + 1);
        id
    }))
}

/// Render `body` as a component of its own, inside the instance that is
/// rendering.
///
/// This is what a capitalised tag becomes. The child's identity is
/// `(parent, call site, keys in force)`, so the same line is the same
/// component on every render, and the same line written twice is two
/// components — exactly the rule a cell follows.
///
/// # Panics
///
/// Outside a [`Scope`]: a component has to be rendered by something.
pub fn component_at<R>(slot: Slot, body: impl FnOnce() -> R) -> R {
    let parent = current().unwrap_or_else(|| {
        panic!(
            "rustx: the component at {}:{}:{} was rendered outside a render. \
             A component function has to be called through the renderer.",
            slot.file, slot.line, slot.column
        )
    });
    let key = (parent, slot, key_path());

    let child = match CHILD_IDS.with(|ids| ids.borrow().get(&key).copied()) {
        Some(child) => child,
        None => {
            // Allocated outside the borrow: the counter is a cell of its
            // own, and reaching it through this map would nest two
            // borrows for no reason.
            let child = next_instance();
            CHILD_IDS.with(|ids| ids.borrow_mut().insert(key, child));
            child
        }
    };

    PARENTS.with(|parents| parents.borrow_mut().insert(child, parent));

    // Declared, so the parent's `end_render` knows it is still wanted.
    FRAMES.with(|frames| {
        if let Some(frame) = frames.borrow_mut().last_mut()
            && !frame.children.contains(&child)
        {
            frame.children.push(child);
        }
    });

    let _scope = Scope::enter(child);
    let rendered = body();
    report_child_inconsistency(end_render());
    rendered
}

/// A child's reach-check complaint, said where a developer will see it.
///
/// It is not returned to the caller: the caller is the parent's body,
/// which is the developer's code, and the complaint is about the
/// child's. The engine prints the parent's; this prints the child's.
fn report_child_inconsistency(outcome: Result<(), String>) {
    if let Err(complaint) = outcome {
        eprintln!("{complaint}");
    }
}

/// The components an instance declared on its last render.
pub fn children_of(instance: InstanceId) -> Vec<InstanceId> {
    CHILDREN.with(|children| {
        children
            .borrow()
            .get(&instance)
            .cloned()
            .unwrap_or_default()
    })
}

/// The instance that has to render for `instance` to be redrawn.
///
/// For a root, itself. For a child, the root above it: a component does
/// not own a region of the page, its root does, so redrawing a child
/// means rendering the root again and letting the diff find what moved.
/// A cycle is impossible — a child's parent is always an instance that
/// was already rendering when the child was made — but the walk is
/// bounded anyway, because a frozen window is a worse failure than a
/// missed redraw.
pub fn root_of(instance: InstanceId) -> InstanceId {
    let mut at = instance;
    for _ in 0..10_000 {
        match PARENTS.with(|parents| parents.borrow().get(&at).copied()) {
            Some(parent) => at = parent,
            None => return at,
        }
    }
    at
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
        mark_dirty(self.owner);
    }
}

/// Say that `id` has to render again on the next frame.
///
/// A cell marks its own owner; a store marks everyone who read it.
pub(crate) fn mark_dirty(id: InstanceId) {
    DIRTY.with(|dirty| {
        let mut dirty = dirty.borrow_mut();
        if !dirty.contains(&id) {
            dirty.push(id);
        }
    });
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
    // Depth first: a child's `Drop` runs before its parent's, the way a
    // tree of values is dropped in Rust.
    let children = CHILDREN.with(|children| children.borrow_mut().remove(&id));
    for child in children.unwrap_or_default() {
        drop_instance(child);
    }
    // The identities of the children it declared go too, or the map
    // would keep a row for every component the application ever showed.
    CHILD_IDS.with(|ids| ids.borrow_mut().retain(|(parent, _, _), _| *parent != id));
    // And the row that names this instance as somebody's child. Pruning
    // by parent alone left one row for every key the application had
    // ever shown — a page visited and left stayed on record for as long
    // as the window was open.
    CHILD_IDS.with(|ids| ids.borrow_mut().retain(|_, child| *child != id));
    PARENTS.with(|parents| parents.borrow_mut().remove(&id));

    REACHED_BEFORE.with(|before| before.borrow_mut().remove(&id));
    let held = CELLS.with(|cells| cells.borrow_mut().remove(&id));
    drop(held);
    DIRTY.with(|dirty| dirty.borrow_mut().retain(|dirty| *dirty != id));

    // Anything else holding this instance's id has to let go of it, or
    // the id would be the one thing a component leaves behind.
    //
    // The list is cloned rather than taken out: a hook is the
    // developer's code once removed — it may ask for a store, release
    // another instance, or panic. Taking the list out would leave it
    // empty for a nested release and gone for good after a panic, and a
    // store that silently stops forgetting unmounted readers is the leak
    // this hook exists to prevent.
    // Each hook is contained, for the same reason the frame loop
    // contains a frame callback: one broken hook must not stop the
    // others from being told, nor take the unmount down with it.
    let hooks: Vec<ForgetHook> = FORGET.with(|hooks| hooks.borrow().clone());
    for hook in &hooks {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hook(id)));
        if outcome.is_err() {
            eprintln!("rustx: a hook watching for released components panicked; carrying on");
        }
    }
}

type ForgetHook = Rc<dyn Fn(InstanceId)>;

thread_local! {
    /// Who else has to be told that an instance is gone.
    static FORGET: RefCell<Vec<ForgetHook>> = const { RefCell::new(Vec::new()) };
}

/// Be told when an instance leaves the tree.
///
/// `state` does not know what else keeps instance ids — the store does,
/// and so will the router. Rather than reach into them from here, they
/// say so themselves, which keeps this module's one job its only job.
pub(crate) fn forget_on_drop(hook: impl Fn(InstanceId) + 'static) {
    FORGET.with(|hooks| hooks.borrow_mut().push(Rc::new(hook)));
}

/// How many instances hold state. For tests and diagnostics.
pub fn live_instances() -> usize {
    CELLS.with(|cells| cells.borrow().len())
}

/// How many child identities are on record. For tests and diagnostics:
/// a number that grows with every key an application has ever shown is
/// a leak, however small each row is.
pub fn live_child_ids() -> usize {
    CHILD_IDS.with(|ids| ids.borrow().len())
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

/// The keys this render opened, as one string. Empty outside any list.
///
/// Keys that were already in force when the render began belong to the
/// component above — see `Frame::keys_at`.
fn key_path() -> String {
    let from = FRAMES.with(|frames| frames.borrow().last().map_or(0, |frame| frame.keys_at));
    KEYS.with(|keys| {
        let keys = keys.borrow();
        keys.get(from..).unwrap_or(&[]).join("\u{1f}")
    })
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

/// What an effect's cell holds: the value it watches, and whatever its
/// last run left open.
type Effect<D, G> = RefCell<(D, Option<G>)>;

/// Run `run` when `watched` is not what it was last render.
///
/// The value `run` returns is a guard. It is held for as long as the
/// watched value stays the same, dropped before the next run, and
/// dropped when the component leaves the tree — so an effect that opens
/// something has nothing to write about closing it.
///
/// There is no dependency array to keep in step with the body: the one
/// value the effect watches is the argument.
///
/// # Panics
///
/// Outside a [`Scope`], for the same reason [`cell`] does.
pub fn effect_at<D: PartialEq + 'static, G: 'static>(
    slot: Slot,
    watched: D,
    run: impl FnOnce() -> G,
) {
    let owner = current().unwrap_or_else(|| {
        panic!(
            "rustx: an effect was asked for at {}:{}:{} outside a render. \
             A component function has to be called through the renderer.",
            slot.file, slot.line, slot.column
        )
    });

    let key = (slot, key_path());
    touch(key.clone());

    // Taken out of the map rather than read through it: `run` is the
    // developer's code and may declare state of its own, which borrows
    // the same map.
    let existing = CELLS.with(|cells| {
        cells
            .borrow()
            .get(&owner)
            .and_then(|slots| slots.get(&key))
            .map(Rc::clone)
    });

    let Some(existing) = existing else {
        let guard = run();
        let memo: Rc<dyn Any> = Rc::new(RefCell::new((watched, Some(guard))));
        CELLS.with(|cells| {
            cells
                .borrow_mut()
                .entry(owner)
                .or_default()
                .insert(key, memo);
        });
        return;
    };

    let memo = existing
        .downcast::<Effect<D, G>>()
        .expect("a slot holds one type, because it is one call");

    if memo.borrow().0 == watched {
        return;
    }

    // Close before opening. A camera does not open twice, so the old
    // guard goes before `run` is called — and it goes outside the borrow,
    // because its `Drop` is the developer's code too.
    let previous = memo.borrow_mut().1.take();
    drop(previous);

    // `watched` is stored only once `run` has returned: an effect that
    // panics leaves the old value behind, so the next render tries again
    // rather than treating the failed run as done.
    let guard = run();
    *memo.borrow_mut() = (watched, Some(guard));
}

thread_local! {
    /// What the previous render of each instance reached.
    static REACHED_BEFORE: RefCell<HashMap<InstanceId, Vec<(Slot, String)>>> =
        RefCell::new(HashMap::new());
}

fn touch(key: (Slot, String)) {
    FRAMES.with(|frames| {
        if let Some(frame) = frames.borrow_mut().last_mut() {
            frame.touched.push(key);
        }
    });
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
    let (reached, declared) = FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        match frames.last_mut() {
            Some(frame) => (
                std::mem::take(&mut frame.touched),
                std::mem::take(&mut frame.children),
            ),
            None => (Vec::new(), Vec::new()),
        }
    });

    // A component this render did not declare is a component that left
    // the tree — a page the router no longer shows. Releasing it here is
    // what makes a navigation drop a page's tasks without anybody
    // unmounting anything by hand.
    let left: Vec<InstanceId> = CHILDREN.with(|children| {
        children
            .borrow()
            .get(&instance)
            .map(|before| {
                before
                    .iter()
                    .filter(|child| !declared.contains(child))
                    .copied()
                    .collect()
            })
            .unwrap_or_default()
    });
    CHILDREN.with(|children| children.borrow_mut().insert(instance, declared));
    for child in left {
        drop_instance(child);
    }

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
    FRAMES.with(|frames| {
        let frames = frames.borrow();
        let mut keys: Vec<String> = frames
            .last()
            .map(|frame| frame.touched.as_slice())
            .unwrap_or_default()
            .iter()
            .filter(|(_, path)| !path.is_empty())
            .map(|(_, path)| path.split('\u{1f}').next().unwrap_or_default().to_string())
            .collect();
        keys.sort();
        keys.dedup();
        keys
    })
}

#[cfg(test)]
mod tests {
    //! The list of "tell me when an instance goes" hooks. It is taken
    //! out of its cell for the call, because a hook may ask for a store
    //! and the store registers hooks — so the putting back has to
    //! survive a hook that panics, or a store silently stops forgetting
    //! unmounted readers for the rest of the process.

    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn a_hook_that_panics_does_not_take_the_other_hooks_with_it() {
        let seen = Rc::new(RefCell::new(Vec::new()));

        forget_on_drop(|_| panic!("a forget hook panicked on purpose"));
        let witness = Rc::clone(&seen);
        forget_on_drop(move |gone| witness.borrow_mut().push(gone));

        let _ = catch_unwind(AssertUnwindSafe(|| drop_instance(InstanceId(10))));
        drop_instance(InstanceId(11));

        assert_eq!(
            *seen.borrow(),
            vec![InstanceId(10), InstanceId(11)],
            "a panicking hook stopped the one standing next to it"
        );
    }

    #[test]
    fn an_instance_dropped_from_inside_a_hook_is_still_announced() {
        // A cell's `Drop` may release another instance — a child, a row
        // of a list. The hooks have to be there for that one too.
        let seen = Rc::new(RefCell::new(Vec::new()));
        let witness = Rc::clone(&seen);

        forget_on_drop(move |gone| {
            witness.borrow_mut().push(gone);
            if gone == InstanceId(20) {
                drop_instance(InstanceId(21));
            }
        });
        drop_instance(InstanceId(20));

        assert_eq!(*seen.borrow(), vec![InstanceId(20), InstanceId(21)]);
    }
}
