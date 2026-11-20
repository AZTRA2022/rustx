//! Re-rendering, once per frame.
//!
//! Mutating state does not draw anything. It marks an instance for the
//! next frame, and the frame does the drawing — all of it, once. That is
//! the promise of §7.4: a capture thread pushing sixty frames a second
//! gets sixty renders, a loop pushing ten thousand updates a second
//! still gets sixty. The developer writes their loop as fast as they
//! like and cannot shoot themselves in the foot. There is no debounce to
//! configure and no throttling API to learn.
//!
//! The ordering that makes it safe is plan 1's: `frame::tick` drains
//! everything worker threads posted **before** it runs the frame
//! callbacks. By the time a render starts, every mutation for this frame
//! has landed, so a render never reads a half-written state.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rustx_dom::Element;
use rustx_dom::diff::{Patch, Path, diff};
use rustx_dom::state::{InstanceId, Scope, drop_instance, end_render, take_dirty};

use crate::dom::{Document, DomError, Node};
use crate::mount::{build_into, root_node, set_attribute};
use crate::tree::{Mounted, element_at};

thread_local! {
    /// Everything mounted on this page, by instance.
    ///
    /// Behind an `Rc` each, because the registry may not be borrowed
    /// while a component renders — see [`each_entry`].
    static ROOTS: RefCell<Vec<Rc<Entry>>> = const { RefCell::new(Vec::new()) };
    static RENDERS: Cell<u64> = const { Cell::new(0) };
    /// Whether this page's one frame callback has been registered.
    ///
    /// A count of roots is not the question: an application whose last
    /// root is released and whose next one is mounted is back at one
    /// root and would have registered a second callback, so the page
    /// would have rendered itself twice per frame from then on.
    static DRAWING: Cell<bool> = const { Cell::new(false) };
}

/// One mounted component, as the registry holds it.
///
/// Two halves, and the split is the point. What identifies the root —
/// its instance and the id of the element it hangs under — never
/// changes, so it can be read while that root is rendering. What a
/// render writes is behind a cell of its own, so taking a root off the
/// page does not have to wait for the render it is inside.
struct Entry {
    id: InstanceId,
    /// The id of the element it hangs under, so it can be taken off.
    id_name: String,
    body: RefCell<Body>,
}

struct Body {
    component: Rc<dyn Fn() -> Element>,
    mounted: Mounted,
}

/// Visit every entry of a registry without holding the registry
/// borrowed while a visit runs.
///
/// This is the shape of a defect rather than a convenience. A render
/// calls the application's components, and — through
/// [`crate::foreign`] — React's own reconciler, which may call back
/// into the engine: a React subtree holding a rustx component mounts a
/// root of its own (`rustx_component`), and a React cleanup releases
/// one (`rustx_release`). Both reach for this registry. Iterating it
/// under `borrow_mut` therefore ended the frame with
/// `already mutably borrowed: BorrowMutError` — a panic in the middle
/// of a render, which leaves the page half-patched.
///
/// So the list of handles is copied under a borrow that is let go
/// before anything is called: `Rc` clones, not entries. A visit may add
/// to the registry, remove from it, or visit a different entry, and
/// what was in the list when the walk began is what the walk visits.
fn each_entry<T>(registry: &RefCell<Vec<Rc<T>>>, mut visit: impl FnMut(&Rc<T>)) {
    let entries: Vec<Rc<T>> = registry.borrow().clone();
    for entry in &entries {
        visit(entry);
    }
}

/// Mount a component under `id` and keep it up to date.
///
/// The component function is called once now and again on any frame
/// where its state changed.
pub fn mount_app(component: impl Fn() -> Element + 'static, id: &str) -> Result<(), DomError> {
    // From `rustx-dom`'s counter, not a second one of our own: a
    // component written inside another is given an instance there, and
    // two numbering schemes would hand the same number to a root and to
    // a child the moment an application had both.
    let started = std::time::Instant::now();
    // A component that never comes back freezes the window, and the
    // thread that would notice is this one. Another thread says so.
    crate::watchdog::watch();
    let _drawing = crate::watchdog::drawing(id);
    let instance = rustx_dom::state::next_instance();
    let root = root_node(id)?;
    let component: Rc<dyn Fn() -> Element> = Rc::new(component);

    let tree = {
        let _scope = Scope::enter(instance);
        let tree = component();
        report_inconsistency(end_render());
        tree
    };
    let mounted = crate::mount::mount_tree(tree, &root)?;
    RENDERS.with(|renders| renders.set(renders.get() + 1));
    // A first mount is a render too, and the number a heavy page is
    // judged by is what the second one costs against it.
    COST.with(|cost| cost.set((started.elapsed().as_micros() as u64, 0)));

    ROOTS.with(|roots| {
        roots.borrow_mut().push(Rc::new(Entry {
            id: instance,
            id_name: id.to_string(),
            body: RefCell::new(Body { component, mounted }),
        }))
    });

    // One frame callback for the whole page, registered on the first
    // mount. Re-registering per root would re-render the page once per
    // root per frame.
    if !DRAWING.with(Cell::get) {
        DRAWING.with(|drawing| drawing.set(true));
        crate::frame::on_frame(render_dirty);
        // And a frame is asked for as soon as anything changes, rather
        // than waited for: a click that draws a tenth of a second later
        // is a click the hand notices.
        rustx_dom::state::when_dirty(crate::frame::wake);
    }
    Ok(())
}

/// How many times a component has been rendered on this page.
pub fn renders() -> u64 {
    RENDERS.with(Cell::get)
}

/// Take a mounted component off the page.
///
/// Its cells go with it, so whatever they held is dropped — a file
/// handle, an open capture session, anything with a `Drop`. That is the
/// cleanup mechanism, and it is Rust's own; the framework does not
/// invent a second one.
pub fn unmount(id: &str) -> Result<bool, DomError> {
    // Taken out of the registry under a borrow that is let go before
    // anything below runs: everything below calls into React or into
    // the application's own `Drop`, and either may reach back here.
    let removed = ROOTS.with(|roots| {
        let mut roots = roots.borrow_mut();
        roots
            .iter()
            .position(|entry| entry.id_name == id)
            .map(|at| roots.remove(at))
    });
    let Some(entry) = removed else {
        return Ok(false);
    };
    // The instance goes whatever else happens: this is where a
    // component's `Drop` runs, so a failure to clear the DOM must not
    // leave its cells — and the tasks they hold — behind.
    let cleared = match entry.body.try_borrow() {
        Ok(body) => {
            // React is told while its containers are still in the
            // document.
            give_back_boundaries(&body.mounted.nodes);
            body.mounted.root.set_text("")
        }
        // Released from inside its own render, which is a React cleanup
        // reaching back through `rustx_release`. The tree it is in the
        // middle of building will be dropped with the entry; the
        // instance is still released.
        Err(_) => Ok(()),
    };
    drop_instance(entry.id);
    cleared?;
    Ok(true)
}

/// Release everything mounted on this page. Runs every component's
/// `Drop`, which is the only cleanup mechanism the framework has — the
/// file a page opened, the capture session it started, the token that
/// stops its background task.
///
/// Called when the page's JavaScript context is released and when the
/// frame queue is shut down. Without it, a window that closed while the
/// process stayed — a second window, `rustx dev` replacing a build —
/// left every root's cells alive: the `Drop`s never ran, and a thread
/// started by `task(…)` went on looking at a flag nobody would ever
/// flip.
///
/// The DOM is not touched. By the time this is called the context the
/// nodes belong to is going or gone, and a handle from a dead context
/// answers [`DomError::ContextDestroyed`] rather than doing anything.
pub fn unmount_all() {
    let entries = ROOTS.with(|roots| std::mem::take(&mut *roots.borrow_mut()));
    for entry in entries {
        drop_instance(entry.id);
    }
}

/// The frame callback. Nothing here runs unless something changed.
fn render_dirty() {
    // A boundary that was rendered before the page had a bridge is
    // handed over here, on the first frame there is somebody to hand it
    // to. Nothing else in this function runs unless something changed.
    crate::foreign::hand_over_pending();

    // The document first, and the marks only once there is somewhere to
    // draw them. `take_dirty` clears what it hands back, so taking the
    // marks before this check threw them away on any frame that ran
    // without a context — and nothing marks an instance twice, so the
    // write was simply never drawn. A page that changed while its
    // context was being replaced stayed as it was, for good.
    let Some(document) = Document::current() else {
        return;
    };

    // A child component that changed is redrawn by rendering the root
    // above it: the root is what owns a host element, and the diff is
    // what finds the one text that moved. Without this step a dirty
    // child matched no root, and its mark was taken and thrown away —
    // a layout's own counter would change in memory and never on screen.
    let dirty: Vec<_> = take_dirty()
        .into_iter()
        .map(rustx_dom::state::root_of)
        .collect();
    if dirty.is_empty() {
        return;
    }

    ROOTS.with(|roots| {
        each_entry(roots, |entry| {
            if !dirty.contains(&entry.id) {
                return;
            }
            // Released while this walk was running — by a React cleanup
            // calling `rustx_release`, or by the application. Its cells
            // are already gone; rendering it again would build them
            // back.
            if !still_mounted(entry.id) {
                return;
            }
            // A render reached from inside a render of the same root.
            // Skipped with a sentence rather than left to `RefCell`,
            // whose panic would end the frame.
            let Ok(mut body) = entry.body.try_borrow_mut() else {
                eprintln!(
                    "rustx: `{}` asked to be drawn from inside its own render; \
                     the frame it was already in will draw it",
                    entry.id_name
                );
                return;
            };
            if let Err(error) = re_render(&document, entry.id, &entry.id_name, &mut body) {
                eprintln!("rustx: a re-render failed: {error}");
            }
        })
    });
}

/// Is this root still on the page?
///
/// Asked without borrowing any entry's body, which is what the split
/// between an [`Entry`] and its [`Body`] is for: the root being asked
/// about is, at the one call site, the one currently rendering.
fn still_mounted(id: InstanceId) -> bool {
    ROOTS.with(|roots| roots.borrow().iter().any(|entry| entry.id == id))
}

fn re_render(
    document: &Document,
    id: InstanceId,
    id_name: &str,
    root: &mut Body,
) -> Result<(), DomError> {
    let started = std::time::Instant::now();
    let _drawing = crate::watchdog::drawing(id_name);
    let fresh = {
        let _scope = Scope::enter(id);
        let fresh = (root.component)();
        // Every render is closed, not only the first. This is where a
        // row that left a keyed list releases its cells, where a child
        // component that is no longer declared is released, and where a
        // kept value that stopped being reached is reported. A render
        // loop that closes only the mount leaves all three unreachable.
        report_inconsistency(end_render());
        fresh
    };
    RENDERS.with(|renders| renders.set(renders.get() + 1));

    let patches = diff(&root.mounted.tree, &fresh);

    // The tree that is on screen is recorded before the patching, not
    // after. A patch that fails leaves the DOM half-changed; recording
    // the new tree only on success would mean the next diff is computed
    // against a tree matching neither the screen nor the state, and the
    // page would drift further apart with every render. Recording it
    // first means the next diff is at least computed against what this
    // render intended.
    let outcome = apply(document, &patches, &fresh, &mut root.mounted);
    root.mounted.tree = fresh;
    COST.with(|cost| cost.set((started.elapsed().as_micros() as u64, patches.len())));
    outcome
}

/// Carry out the corrections the diff asked for.
///
/// This is the only place that owns both the new tree and the DOM, which
/// is why a patch names a path into the new tree rather than carrying a
/// subtree it could not clone.
pub fn apply(
    document: &Document,
    patches: &[Patch],
    fresh: &Element,
    mounted: &mut Mounted,
) -> Result<(), DomError> {
    for patch in patches {
        match patch {
            Patch::SetText { path, text } => {
                if let Some(node) = mounted.dom_at(path) {
                    node.set_text(text)?;
                }
            }
            Patch::SetAttribute { path, name, value } => {
                if let Some(node) = mounted.dom_at(path) {
                    set_attribute(node, name, value)?;
                }
            }
            Patch::RemoveAttribute { path, name } => {
                if let Some(node) = mounted.dom_at(path) {
                    node.remove_attribute(name)?;
                }
            }
            Patch::ReplaceHandlers { path, source } => {
                replace_handlers(path, source, fresh, mounted);
            }
            // The props of a node React owns. There is nobody to hand
            // them to until the bridge exists, and a correction that is
            // quietly dropped is worse than one that says so.
            Patch::ForeignProps { path, node: tree } => {
                // The container is what is mounted; the tree is what the
                // render wants in it. A boundary that has no container
                // any more is a boundary nobody can be handed.
                if let Some(node) = mounted.at(path)
                    && node.foreign.is_some()
                    && let Some(dom) = node.dom.clone()
                {
                    crate::foreign::hand_over(tree, &dom);
                }
            }
            Patch::Replace { path, source } => {
                replace(document, path, source, fresh, mounted)?;
            }
            Patch::Insert {
                parent,
                index,
                source,
            } => {
                insert(document, parent, *index, source, fresh, mounted)?;
            }
            Patch::Remove { parent, index } => {
                remove(parent, *index, mounted)?;
            }
            Patch::Move { parent, from, to } => {
                move_child(parent, *from, *to, mounted)?;
            }
        }
    }
    Ok(())
}

/// A write to a cell, not a conversation with the DOM: the listener
/// posted at mount reads whatever is in the slot when it fires.
fn replace_handlers(path: &Path, source: &Path, fresh: &Element, mounted: &mut Mounted) {
    let Some(Element::Tag(tag)) = element_at(fresh, source) else {
        return;
    };
    // A handler the node did not have at mount has no slot and no
    // listener, so it needs both — otherwise a button that gains its
    // `onClick` on a later render is simply dead, which looks like the
    // handler is wrong rather than missing.
    let missing: Vec<String> = match mounted.at(path) {
        Some(node) => tag
            .handlers
            .iter()
            .map(|(name, _)| name.clone())
            .filter(|name| !node.handlers.iter().any(|(known, _)| known == name))
            .collect(),
        None => return,
    };
    for name in missing {
        let Some(dom) = mounted.at(path).and_then(|node| node.dom.clone()) else {
            continue;
        };
        match crate::mount::listen_later(&dom, &name) {
            Ok(slot) => {
                if let Some(node) = mounted.at_mut(path) {
                    node.handlers.push((name, slot));
                }
            }
            Err(error) => eprintln!("rustx: could not attach {name}: {error}"),
        }
    }

    let Some(node) = mounted.at_mut(path) else {
        return;
    };
    for (name, slot) in &node.handlers {
        let replacement = tag
            .handlers
            .iter()
            .find(|(fresh_name, _)| fresh_name == name)
            .map(|(_, handler)| handler.clone());
        *slot.borrow_mut() = replacement;
    }
}

fn replace(
    document: &Document,
    path: &Path,
    source: &Path,
    fresh: &Element,
    mounted: &mut Mounted,
) -> Result<(), DomError> {
    let Some(element) = element_at(fresh, source) else {
        return Ok(());
    };
    let Some(parent_path) = parent_of(path) else {
        // Replacing the root: clear it and build again.
        give_back_boundaries(&mounted.nodes);
        mounted.root.set_text("")?;
        mounted.nodes = build_into(document, element, &mounted.root.clone())?;
        return Ok(());
    };
    let index = *path.0.last().expect("a non-root path has a last index");

    let parent_dom = mounted
        .dom_at(&parent_path)
        .cloned()
        .unwrap_or_else(|| mounted.root.clone());
    let built = build_into(document, element, &parent_dom)?;

    if let Some(old) = mounted.at(path) {
        // Before it leaves the document, not after.
        give_back_boundaries(old);
        detach(old)?;
    }
    if let Some(new) = built.dom.clone() {
        place(&parent_dom, &new, index, mounted, &parent_path)?;
    }
    if let Some(parent) = mounted.at_mut(&parent_path)
        && let Some(slot) = parent.children.get_mut(index)
    {
        *slot = built;
    }
    Ok(())
}

fn insert(
    document: &Document,
    parent: &Path,
    index: usize,
    source: &Path,
    fresh: &Element,
    mounted: &mut Mounted,
) -> Result<(), DomError> {
    let Some(element) = element_at(fresh, source) else {
        return Ok(());
    };
    let parent_dom = mounted
        .dom_at(parent)
        .cloned()
        .unwrap_or_else(|| mounted.root.clone());
    let built = build_into(document, element, &parent_dom)?;
    let new = built.dom.clone();

    // The bookkeeping is updated before the node is placed, because
    // `place` reads the sibling that should follow out of it. Placing
    // first would make it read the node this one has to go *before*,
    // and the new child would be appended at the end instead — leaving
    // the DOM in an order the bookkeeping does not describe, and every
    // later path-addressed patch aimed at the wrong node.
    let index = match mounted.at_mut(parent) {
        Some(node) => {
            let index = index.min(node.children.len());
            node.children.insert(index, built);
            index
        }
        None => index,
    };

    if let Some(new) = new {
        place(&parent_dom, &new, index, mounted, parent)?;
    }
    Ok(())
}

fn remove(parent: &Path, index: usize, mounted: &mut Mounted) -> Result<(), DomError> {
    let gone = mounted
        .at_mut(parent)
        .filter(|node| index < node.children.len())
        .map(|node| node.children.remove(index));
    if let Some(gone) = gone {
        give_back_boundaries(&gone);
        detach(&gone)?;
    }
    Ok(())
}

/// Take everything a subtree put in the document out of it.
///
/// Not `node.dom.remove()`. A fragment and an `Empty` have no node of
/// their own, so that call removed nothing at all and left every
/// element the subtree had put on the page in the document — with
/// nothing in the bookkeeping pointing at it any more, so no later
/// render could find it either. A layout's page is one of the layout's
/// children, which is why the symptom was "leaving a nested layout
/// leaves its last page's DOM behind", once per navigation, for as long
/// as the window stayed open.
fn detach(node: &crate::tree::MountedNode) -> Result<(), DomError> {
    let mut attached = Vec::new();
    node.attached(&mut attached);
    for dom in attached {
        dom.remove()?;
    }
    Ok(())
}

fn move_child(
    parent: &Path,
    from: usize,
    to: usize,
    mounted: &mut Mounted,
) -> Result<(), DomError> {
    let parent_dom = mounted
        .dom_at(parent)
        .cloned()
        .unwrap_or_else(|| mounted.root.clone());

    let Some(node) = mounted.at_mut(parent) else {
        return Ok(());
    };
    if from >= node.children.len() {
        return Ok(());
    }
    let moved = node.children.remove(from);
    let to = to.min(node.children.len());
    let moved_dom = moved.dom.clone();
    node.children.insert(to, moved);

    if let Some(dom) = moved_dom {
        place(&parent_dom, &dom, to, mounted, parent)?;
    }
    Ok(())
}

/// Put `node` at child position `index` of `parent_dom`.
///
/// **Invariant:** the bookkeeping already holds this node at `index`,
/// so the child that should follow it is at `index + 1`. All three
/// callers arrange that before calling — `replace` because the old node
/// still occupies the slot, `move_child` and `insert` because they
/// update the list first.
///
/// `insertBefore` with a null reference appends, which is exactly what
/// is wanted past the end. A sibling with no DOM node of its own — a
/// fragment, or an `Empty` — is skipped over rather than treated as the
/// end of the list.
fn place(
    parent_dom: &Node,
    node: &Node,
    index: usize,
    mounted: &Mounted,
    parent: &Path,
) -> Result<(), DomError> {
    // The first node of the first following sibling that has any —
    // `first_attached` and not `dom`, because a fragment has no node of
    // its own and skipping it would put this child *after* everything
    // the fragment holds rather than before it.
    let before = mounted.at(parent).and_then(|parent| {
        parent
            .children
            .iter()
            .skip(index + 1)
            .find_map(crate::tree::MountedNode::first_attached)
    });
    parent_dom.insert_before(node, before.as_ref())
}

fn parent_of(path: &Path) -> Option<Path> {
    if path.0.is_empty() {
        return None;
    }
    Some(Path(path.0[..path.0.len() - 1].to_vec()))
}

/// A render that stopped reaching one of its kept values is a bug the
/// developer can fix, and a silent one they never will. It is printed
/// rather than raised: the page is still correct enough to look at, and
/// a panic here would take the window down over a diagnostic.
fn report_inconsistency(outcome: Result<(), String>) {
    if let Err(complaint) = outcome {
        eprintln!("{complaint}");
    }
}

/// Tell React about every boundary in this subtree, before any of it is
/// detached.
///
/// The order is the point. A React tree told after its container has
/// left the document never runs its cleanups, and its listeners stay
/// attached to a node nobody can reach.
fn give_back_boundaries(node: &crate::tree::MountedNode) {
    let mut boundaries = Vec::new();
    node.boundaries(&mut boundaries);
    for (_, container) in boundaries {
        crate::foreign::give_back(&container);
    }
}

/// How many components are mounted on this page. For tests and
/// diagnostics: a root that was released and still counted is a leak.
pub fn roots() -> usize {
    ROOTS.with(|roots| roots.borrow().len())
}

thread_local! {
    /// What the last render cost, and how many corrections it made.
    static COST: Cell<(u64, usize)> = const { Cell::new((0, 0)) };
}

/// How long the last render took, in microseconds, and how many patches
/// it applied.
///
/// A heavy page is the case this exists for: a thousand rows on screen
/// and one of them changing has to cost one correction, not a thousand,
/// and a developer who suspects otherwise should be able to read the
/// number instead of guessing from a stopwatch.
pub fn last_render() -> (u64, usize) {
    COST.with(Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The registry walk, with the two things a real visit does: a
    /// React subtree that mounts a rustx component adds a root, and a
    /// React cleanup that releases one removes a root. Both happen
    /// *inside* `hand_over`, which happens inside a render — so under
    /// the walk.
    ///
    /// Walking the registry under `borrow_mut`, which is how this was
    /// written, panicked with `already mutably borrowed: BorrowMutError`
    /// on either of them: a panic in the middle of a frame, which
    /// leaves the page half-patched and the window with no explanation.
    #[test]
    fn a_visit_may_add_to_the_registry_and_take_from_it() {
        let registry: RefCell<Vec<Rc<RefCell<u32>>>> =
            RefCell::new(vec![Rc::new(RefCell::new(1)), Rc::new(RefCell::new(2))]);

        let mut seen = Vec::new();
        each_entry(&registry, |entry| {
            seen.push(*entry.borrow());
            // A root mounted from inside the render.
            registry.borrow_mut().push(Rc::new(RefCell::new(99)));
            // And one released from inside it.
            registry.borrow_mut().remove(0);
        });

        assert_eq!(
            seen,
            vec![1, 2],
            "the walk must visit what was there when it began"
        );
        assert_eq!(
            registry.borrow().len(),
            2,
            "two added and two removed leaves the registry as it was"
        );
    }

    /// And a visit may reach another entry, which a render does
    /// whenever one root's component asks a second root to redraw.
    #[test]
    fn a_visit_may_reach_the_other_entries() {
        let registry: RefCell<Vec<Rc<RefCell<u32>>>> =
            RefCell::new(vec![Rc::new(RefCell::new(1)), Rc::new(RefCell::new(2))]);

        let mut seen = Vec::new();
        each_entry(&registry, |entry| {
            let mine = *entry.borrow();
            let all: u32 = registry.borrow().iter().map(|other| *other.borrow()).sum();
            seen.push((mine, all));
        });

        assert_eq!(
            seen,
            vec![(1, 3), (2, 3)],
            "a visit must be able to look at the whole registry"
        );
    }
}
