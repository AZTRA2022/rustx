//! What is on screen, remembered.
//!
//! Plan 2's mounting wrote into the DOM and forgot. Re-rendering cannot:
//! to change only what changed, something has to hold the tree that was
//! drawn and the DOM node each of its elements became.
//!
//! # Why handlers live in a slot
//!
//! `addEventListener` has no undo worth using — removing a listener
//! needs the very same function object, and a re-render builds a new
//! closure every time. So a node gets **one** listener per event, for
//! as long as it lives, and that listener reads the current handler out
//! of a slot. Replacing a handler is then a write to a cell, not a
//! conversation with the DOM, and a stale closure cannot survive because
//! nothing holds it any more.

use std::cell::RefCell;
use std::rc::Rc;

use rustx_dom::diff::Path;
use rustx_dom::{Element, Handler};

use crate::dom::Node;

/// The handler a node's listener will call next time it fires.
pub(crate) type HandlerSlot = Rc<RefCell<Option<Handler>>>;

/// One element of the tree that is on screen.
pub struct MountedNode {
    /// The DOM node, or `None` for a fragment or an `Empty`, which have
    /// no node of their own.
    pub(crate) dom: Option<Node>,
    pub(crate) handlers: Vec<(String, HandlerSlot)>,
    pub(crate) children: Vec<MountedNode>,
}

impl MountedNode {
    pub(crate) fn empty() -> Self {
        Self {
            dom: None,
            handlers: Vec::new(),
            children: Vec::new(),
        }
    }
}

/// A tree that is on screen, and the element tree it was built from.
pub struct Mounted {
    /// The parent everything hangs under.
    pub(crate) root: Node,
    /// What was rendered last. The next render is compared against it.
    pub(crate) tree: Element,
    pub(crate) nodes: MountedNode,
}

impl Mounted {
    /// The mounted node at this path, if the path names one.
    pub(crate) fn at(&self, path: &Path) -> Option<&MountedNode> {
        let mut node = &self.nodes;
        for index in &path.0 {
            node = node.children.get(*index)?;
        }
        Some(node)
    }

    pub(crate) fn at_mut(&mut self, path: &Path) -> Option<&mut MountedNode> {
        let mut node = &mut self.nodes;
        for index in &path.0 {
            node = node.children.get_mut(*index)?;
        }
        Some(node)
    }

    /// The DOM node a path names, walking up past fragments — which have
    /// no node — to the nearest ancestor that has one.
    pub(crate) fn dom_at(&self, path: &Path) -> Option<&Node> {
        let mut node = &self.nodes;
        let mut found = node.dom.as_ref();
        for index in &path.0 {
            node = node.children.get(*index)?;
            if node.dom.is_some() {
                found = node.dom.as_ref();
            }
        }
        found
    }

    /// The element at this path in the tree that is on screen.
    pub fn element_at(&self) -> &Element {
        &self.tree
    }
}

/// The element at `path` inside `tree`.
pub(crate) fn element_at<'a>(tree: &'a Element, path: &Path) -> Option<&'a Element> {
    let mut element = tree;
    for index in &path.0 {
        element = match element {
            Element::Tag(tag) => tag.children.get(*index)?,
            Element::Fragment(children) => children.get(*index)?,
            _ => return None,
        };
    }
    Some(element)
}
