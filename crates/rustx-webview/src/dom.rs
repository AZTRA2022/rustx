//! The DOM, from Rust, as a list of commands for the page.
//!
//! The application's Rust code and the page no longer share a process, so
//! a write cannot be a call. It is a command, appended here and sent with
//! all the others the frame wrote, in order, as one message
//! ([`take_ops`]). A [`Node`] is an id the host made up; the page keeps the
//! table from ids to real nodes. An id is never reused, so a handle that
//! outlives its node can only be refused, never aimed at a stranger.
//!
//! Reading the DOM is the part that does not survive the move: the answer
//! is a message away. The reads the CEF engine offered (`text`,
//! `attribute`, `query_selector`, `count_matching`) answer a typed error
//! that says so. No application in this repository used them.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use serde_json::{Value as Json, json};

use crate::value::NativeError;

thread_local! {
    /// The commands written since the last frame was sent.
    static OPS: RefCell<Vec<Json>> = const { RefCell::new(Vec::new()) };
    static NEXT: Cell<u64> = const { Cell::new(1) };
    /// Whether the page has said it is there. Until then there is no
    /// document, and `Document::current()` says so.
    static READY: Cell<bool> = const { Cell::new(false) };
    /// Bumped every time the page goes away: handles made before are stale.
    static GENERATION: Cell<u64> = const { Cell::new(0) };
    /// The ids the page's document had when it was ready, plus the ones
    /// this side has given an element since.
    static PAGE_IDS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    /// The last input sequence number the host has handled, per node.
    /// Written back with the value of a field, so the page can tell a
    /// stale value from a fresh one while somebody is typing.
    static ACKS: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
}

/// What can go wrong while touching the DOM.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomError {
    /// The handle belongs to a page that no longer exists.
    ContextDestroyed,
    /// There is no page to write to yet.
    NoContext,
    /// The operation is refused, or cannot be answered from this side.
    Failed(String),
}

impl std::fmt::Display for DomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContextDestroyed => f.write_str("context destroyed"),
            Self::NoContext => f.write_str("no page to write to yet"),
            Self::Failed(what) => write!(f, "DOM operation failed: {what}"),
        }
    }
}

impl std::error::Error for DomError {}

impl From<DomError> for NativeError {
    fn from(error: DomError) -> Self {
        NativeError::new(error.to_string())
    }
}

/// The page is there, with these ids in its document.
pub(crate) fn page_ready(ids: Vec<String>) {
    READY.with(|ready| ready.set(true));
    PAGE_IDS.with(|held| *held.borrow_mut() = ids.into_iter().collect());
}

/// The page went away: what was written for it is dropped, and every
/// handle made so far is stale.
pub(crate) fn page_gone() {
    READY.with(|ready| ready.set(false));
    GENERATION.with(|generation| generation.set(generation.get() + 1));
    OPS.with(|ops| ops.borrow_mut().clear());
    PAGE_IDS.with(|held| held.borrow_mut().clear());
    ACKS.with(|held| held.borrow_mut().clear());
}

/// The host handled input number `seq` of this node.
pub(crate) fn acknowledge(node: u64, seq: u64) {
    ACKS.with(|held| {
        let mut held = held.borrow_mut();
        let last = held.entry(node).or_insert(0);
        *last = (*last).max(seq);
    });
}

/// Everything written since the last call, as the text of a JSON list, or
/// `None` when nothing was written.
pub(crate) fn take_ops() -> Option<String> {
    let ops = OPS.with(|ops| std::mem::take(&mut *ops.borrow_mut()));
    (!ops.is_empty()).then(|| Json::Array(ops).to_string())
}

/// The answer to a native the page called, by the id the page gave it.
/// It travels with the frame's other commands, so it reaches the page
/// after anything the native wrote.
pub(crate) fn reply(id: u64, outcome: Result<crate::value::Value, String>) {
    let op = match outcome {
        Ok(value) => match crate::protocol::value_to_json(&value, 0) {
            Ok(json) => json!(["rp", id, true, json]),
            Err(error) => json!(["rp", id, false, error.to_string()]),
        },
        Err(message) => json!(["rp", id, false, message]),
    };
    push(op);
}

fn push(op: Json) {
    OPS.with(|ops| ops.borrow_mut().push(op));
}

fn next_id() -> u64 {
    NEXT.with(|next| {
        let id = next.get();
        next.set(id + 1);
        id
    })
}

fn generation() -> u64 {
    GENERATION.with(Cell::get)
}

fn asynchronous(what: &str) -> DomError {
    DomError::Failed(format!(
        "`{what}` cannot answer here: the page is in another process, so reading it is a \
         message away. Keep the value in the application's own state instead"
    ))
}

/// A handle on a node of the page.
#[derive(Clone)]
pub struct Node {
    id: u64,
    tag: Option<Rc<str>>,
    generation: u64,
}

/// The document of the page.
#[derive(Clone)]
pub struct Document {
    _private: (),
}

impl Document {
    /// The document, or `None` until the page has said it is ready, and
    /// again as soon as it goes away.
    pub fn current() -> Option<Self> {
        READY.with(Cell::get).then_some(Self { _private: () })
    }

    /// The element carrying this `id`, if the page has one.
    pub fn get_element_by_id(&self, id: &str) -> Result<Option<Node>, DomError> {
        if !PAGE_IDS.with(|held| held.borrow().contains(id)) {
            return Ok(None);
        }
        let node = self.fresh(None);
        push(json!(["by", node.id, id]));
        Ok(Some(node))
    }

    /// A new, unattached element.
    pub fn create_element(&self, tag: &str) -> Result<Node, DomError> {
        let node = self.fresh(Some(tag.to_ascii_uppercase()));
        push(json!(["el", node.id, tag]));
        Ok(node)
    }

    pub fn create_text_node(&self, text: &str) -> Result<Node, DomError> {
        let node = self.fresh(Some("#text".to_string()));
        push(json!(["tx", node.id, text]));
        Ok(node)
    }

    pub fn count_matching(&self, _selector: &str) -> Result<usize, DomError> {
        Err(asynchronous("count_matching"))
    }

    pub fn query_selector(&self, _selector: &str) -> Result<Option<Node>, DomError> {
        Err(asynchronous("query_selector"))
    }

    fn fresh(&self, tag: Option<String>) -> Node {
        Node {
            id: next_id(),
            tag: tag.map(Rc::from),
            generation: generation(),
        }
    }
}

impl Node {
    /// The id the page knows this node by.
    pub fn id(&self) -> u64 {
        self.id
    }

    fn alive(&self) -> Result<(), DomError> {
        match self.generation == generation() {
            true => Ok(()),
            false => Err(DomError::ContextDestroyed),
        }
    }

    pub fn set_text(&self, text: &str) -> Result<(), DomError> {
        self.alive()?;
        push(json!(["st", self.id, text]));
        Ok(())
    }

    pub fn text(&self) -> Result<String, DomError> {
        Err(asynchronous("text"))
    }

    pub fn attribute(&self, _name: &str) -> Result<Option<String>, DomError> {
        Err(asynchronous("attribute"))
    }

    pub fn set_attribute(&self, name: &str, value: &str) -> Result<(), DomError> {
        self.alive()?;
        if name == "id" {
            PAGE_IDS.with(|held| held.borrow_mut().insert(value.to_string()));
        }
        push(json!(["sa", self.id, name, value]));
        Ok(())
    }

    pub fn remove_attribute(&self, name: &str) -> Result<(), DomError> {
        self.alive()?;
        push(json!(["ra", self.id, name]));
        Ok(())
    }

    /// The tag, upper case, as the DOM writes it. Empty for a node taken
    /// by id, whose tag this side never learned.
    pub fn tag_name(&self) -> Result<String, DomError> {
        self.alive()?;
        Ok(self.tag.as_deref().unwrap_or("").to_string())
    }

    /// A live property of a form element, such as `value`.
    ///
    /// `value` carries the last input number the host has handled for this
    /// node: a field somebody is still typing in has newer input than that,
    /// and the page then keeps what is in the field.
    pub fn set_text_property(&self, name: &str, text: &str) -> Result<(), DomError> {
        self.alive()?;
        let ack = ACKS.with(|held| held.borrow().get(&self.id).copied().unwrap_or(0));
        push(json!(["sp", self.id, name, text, ack]));
        Ok(())
    }

    pub fn set_flag_property(&self, name: &str, whether: bool) -> Result<(), DomError> {
        self.alive()?;
        push(json!(["sp", self.id, name, whether, 0]));
        Ok(())
    }

    pub fn append_child(&self, child: &Node) -> Result<(), DomError> {
        self.alive()?;
        push(json!(["ap", self.id, child.id]));
        Ok(())
    }

    pub fn insert_before(&self, child: &Node, before: Option<&Node>) -> Result<(), DomError> {
        self.alive()?;
        push(json!(["ib", self.id, child.id, before.map(|node| node.id)]));
        Ok(())
    }

    /// Take the node out of the document. The page forgets its id and the
    /// ids of everything below it.
    pub fn remove(&self) -> Result<(), DomError> {
        self.alive()?;
        push(json!(["rm", self.id]));
        Ok(())
    }

    /// Ask the page to tell the host when this node gets an event of
    /// `kind`.
    pub(crate) fn listen(&self, kind: &str) -> Result<(), DomError> {
        self.alive()?;
        push(json!(["on", self.id, kind]));
        Ok(())
    }
}
