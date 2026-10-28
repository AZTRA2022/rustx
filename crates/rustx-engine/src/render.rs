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

use rustx_dom::diff::{Patch, Path, diff};
use rustx_dom::state::{InstanceId, Scope, drop_instance, take_dirty};
use rustx_dom::Element;

use crate::dom::{Document, DomError, Node};
use crate::mount::{build_into, root_node, set_attribute};
use crate::tree::{Mounted, element_at};

thread_local! {
    /// Everything mounted on this page, by instance.
    static ROOTS: RefCell<Vec<Root>> = const { RefCell::new(Vec::new()) };
    static RENDERS: Cell<u64> = const { Cell::new(0) };
    static NEXT_INSTANCE: Cell<u64> = const { Cell::new(1) };
}

struct Root {
    id: InstanceId,
    /// The id of the element it hangs under, so it can be taken off.
    id_name: String,
    component: Rc<dyn Fn() -> Element>,
    mounted: Mounted,
}

/// Mount a component under `id` and keep it up to date.
///
/// The component function is called once now and again on any frame
/// where its state changed.
pub fn mount_app(component: impl Fn() -> Element + 'static, id: &str) -> Result<(), DomError> {
    let instance = InstanceId(NEXT_INSTANCE.with(|next| {
        let id = next.get();
        next.set(id + 1);
        id
    }));
    let root = root_node(id)?;
    let component: Rc<dyn Fn() -> Element> = Rc::new(component);

    let tree = {
        let _scope = Scope::enter(instance);
        component()
    };
    let mounted = crate::mount::mount_tree(tree, &root)?;
    RENDERS.with(|renders| renders.set(renders.get() + 1));

    ROOTS.with(|roots| {
        roots.borrow_mut().push(Root {
            id: instance,
            id_name: id.to_string(),
            component,
            mounted,
        })
    });

    // One frame callback for the whole page, registered on the first
    // mount. Re-registering per root would re-render the page once per
    // root per frame.
    if ROOTS.with(|roots| roots.borrow().len()) == 1 {
        crate::frame::on_frame(render_dirty);
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
    let removed = ROOTS.with(|roots| {
        let mut roots = roots.borrow_mut();
        roots
            .iter()
            .position(|root| root.id_name == id)
            .map(|at| roots.remove(at))
    });
    let Some(root) = removed else {
        return Ok(false);
    };
    root.mounted.root.set_text("")?;
    drop_instance(root.id);
    Ok(true)
}

/// Release everything mounted on this page. Runs every component's
/// `Drop`, which is the only cleanup mechanism the framework has.
pub fn unmount_all() {
    let roots = ROOTS.with(|roots| std::mem::take(&mut *roots.borrow_mut()));
    for root in roots {
        drop_instance(root.id);
    }
}

/// The frame callback. Nothing here runs unless something changed.
fn render_dirty() {
    let dirty = take_dirty();
    if dirty.is_empty() {
        return;
    }
    let Some(document) = Document::current() else {
        return;
    };

    ROOTS.with(|roots| {
        let mut roots = roots.borrow_mut();
        for root in roots.iter_mut() {
            if !dirty.contains(&root.id) {
                continue;
            }
            if let Err(error) = re_render(&document, root) {
                eprintln!("rustx: a re-render failed: {error}");
            }
        }
    });
}

fn re_render(document: &Document, root: &mut Root) -> Result<(), DomError> {
    let fresh = {
        let _scope = Scope::enter(root.id);
        (root.component)()
    };
    RENDERS.with(|renders| renders.set(renders.get() + 1));

    let patches = diff(&root.mounted.tree, &fresh);
    apply(document, &patches, &fresh, &mut root.mounted)?;
    root.mounted.tree = fresh;
    Ok(())
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

    if let Some(old) = mounted.at(path).and_then(|node| node.dom.clone()) {
        old.remove()?;
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

    if let Some(new) = built.dom.clone() {
        place(&parent_dom, &new, index, mounted, parent)?;
    }
    if let Some(node) = mounted.at_mut(parent) {
        let index = index.min(node.children.len());
        node.children.insert(index, built);
    }
    Ok(())
}

fn remove(parent: &Path, index: usize, mounted: &mut Mounted) -> Result<(), DomError> {
    let gone = mounted
        .at_mut(parent)
        .filter(|node| index < node.children.len())
        .map(|node| node.children.remove(index));
    if let Some(gone) = gone
        && let Some(dom) = gone.dom
    {
        dom.remove()?;
    }
    Ok(())
}

fn move_child(parent: &Path, from: usize, to: usize, mounted: &mut Mounted) -> Result<(), DomError> {
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
/// `insertBefore` with a null reference appends, which is exactly what
/// is wanted past the end.
fn place(
    parent_dom: &Node,
    node: &Node,
    index: usize,
    mounted: &Mounted,
    parent: &Path,
) -> Result<(), DomError> {
    let before = mounted
        .at(parent)
        .and_then(|parent| parent.children.get(index + 1))
        .and_then(|sibling| sibling.dom.clone());
    parent_dom.insert_before(node, before.as_ref())
}

fn parent_of(path: &Path) -> Option<Path> {
    if path.0.is_empty() {
        return None;
    }
    Some(Path(path.0[..path.0.len() - 1].to_vec()))
}
