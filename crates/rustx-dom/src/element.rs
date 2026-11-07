//! The element tree, and what may be written inside `{}`.

use std::fmt::Display;
use std::rc::Rc;

/// A node of the tree a component returns.
#[derive(Debug, Clone)]
pub enum Element {
    Tag(TagElement),
    Text(String),
    /// Siblings with no wrapping node — what `<>…</>` produces.
    Fragment(Vec<Element>),
    /// Nothing at all. A component may legitimately render nothing.
    Empty,
    /// A node React owns.
    ///
    /// rustx puts the container on the page, says which React component
    /// goes in it and with what props, and then does not look inside:
    /// the diff treats it as opaque, because two reconcilers reaching
    /// into one subtree is a bug nobody can debug. The `id` is given by
    /// the compiler, deterministically, the way a `Slot` is — so the
    /// engine finds the container without searching for it by position.
    Foreign {
        /// The component, its props and whatever is written inside it.
        node: ForeignNode,
        id: String,
    },
}

/// A component React owns, as the page wrote it.
///
/// It carries its children, because a library's components are written
/// inside one another — a chart holds its axes and its lines — and the
/// whole tree is handed over at once. They are not `Element`s: what is
/// inside a React tree is React's, and a DOM node rustx thinks it owns
/// in a tree React reconciles is the one thing this boundary exists to
/// prevent. The type is what says so.
#[derive(Debug, Clone)]
pub struct ForeignNode {
    /// `"recharts::LineChart"`: the module, then the export.
    pub component: String,
    pub props: Vec<(String, crate::Value)>,
    /// The props that are functions: `onValueChange`, `onSelect`, every
    /// `on…` a library calls when something happens inside it.
    pub callbacks: Vec<(String, Callback)>,
    pub children: Vec<ForeignChild>,
}

/// Two nodes are the same node when they name the same component with
/// the same props and the same children.
///
/// Callbacks are left out, and have to be: a closure is built afresh on
/// every render, so comparing them would make every boundary different
/// from itself every frame. What a callback does is run — it is not
/// something the screen shows.
impl PartialEq for ForeignNode {
    fn eq(&self, other: &Self) -> bool {
        self.component == other.component
            && self.props == other.props
            && self.children == other.children
    }
}

/// What a library calls when something happens inside it.
///
/// The arguments are whatever the library passed, as values: a string
/// for `onValueChange`, a number for a slider, nothing for a close
/// button. What cannot cross — an event object, a DOM node, another
/// function — arrives as `Value::Null`, because a callback that refused
/// the call would be a library that stops working.
#[derive(Clone)]
pub struct Callback(Called);

/// What a library was given, held where it can be called again.
type Called = Rc<dyn Fn(&[crate::Value]) + 'static>;

impl Callback {
    pub fn new(callback: impl Fn(&[crate::Value]) + 'static) -> Self {
        Self(Rc::new(callback))
    }

    pub fn call(&self, arguments: &[crate::Value]) {
        (self.0)(arguments)
    }

    /// The first argument, which is what a library almost always passes.
    pub fn first(arguments: &[crate::Value]) -> crate::Value {
        Self::nth(arguments, 0)
    }

    /// The argument at `at`, or `Null` when the library passed fewer.
    ///
    /// Null rather than a panic, for the same reason the whole boundary
    /// is forgiving: a library that calls a prop with one argument
    /// where its own documentation says two is a library that keeps
    /// working here, with the second reading as nothing.
    pub fn nth(arguments: &[crate::Value], at: usize) -> crate::Value {
        arguments.get(at).cloned().unwrap_or(crate::Value::Null)
    }
}

/// Dire a une closure que son parametre est un [`Event`].
///
/// Pourquoi une fonction qui rend ce qu'on lui donne : une closure
/// ecrite `move |event| …` dans un `let` est verifiee avant d'etre
/// appelee, donc Rust ne connait pas encore le type de `event` quand il
/// lit `event.value` — `type annotations needed`, sur un fichier que
/// personne n'a ecrit. Le passer par cette fonction donne l'attente qui
/// manquait : la borne dit `Fn(Event)`, et la closure l'apprend de la.
///
/// `Fn` et non `FnMut`, parce que c'est ce qu'un [`Callback`] prend :
/// un gestionnaire qui ecrit l'etat le fait a travers une cellule, et
/// une cellule s'ecrit sans `&mut`.
pub fn taking_an_event<F: Fn(Event)>(handler: F) -> F {
    handler
}

impl std::fmt::Debug for Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Callback(..)")
    }
}

/// What may be written inside a component React owns.
#[derive(Debug, Clone)]
pub enum ForeignChild {
    Node(ForeignNode),
    Text(String),
    /// What `{…}` in the page came to.
    ///
    /// A library whose content is its children — a markdown renderer, a
    /// code viewer, a button's label — is given a value, and React
    /// renders a string, a number or a list of them as its children
    /// exactly as it renders an element.
    Value(crate::Value),
    /// A component of the page, written inside a component of a library.
    ///
    /// `<Card>{Resume()}</Card>`: the thing between the braces is an
    /// `Element`, which is neither a string nor a number nor a node of
    /// React's. It used to be nothing at all — `the trait bound
    /// rustx::Element: ToValue is not satisfied`, about the one line
    /// that mixes the two halves of this framework, which is the whole
    /// reason the framework exists.
    ///
    /// It is **described** rather than hosted: a tag becomes the same
    /// `{ component, props, children }` that a native tag below a
    /// boundary already becomes, so React builds it and reconciles it.
    /// That is not a concession — everything below a boundary is
    /// already React's, and a DOM node rustx thought it owned inside a
    /// tree React reconciles is the one thing the boundary exists to
    /// prevent.
    Element(Box<Element>),
}

/// Two children are the same child when they show the same thing.
///
/// Handlers are left out, for the reason `ForeignNode` leaves its
/// callbacks out: a closure is built afresh on every render, so
/// comparing them would make every boundary differ from itself every
/// frame and React would be handed a new tree for nothing.
impl PartialEq for ForeignChild {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ForeignChild::Node(ours), ForeignChild::Node(theirs)) => ours == theirs,
            (ForeignChild::Text(ours), ForeignChild::Text(theirs)) => ours == theirs,
            (ForeignChild::Value(ours), ForeignChild::Value(theirs)) => ours == theirs,
            (ForeignChild::Element(ours), ForeignChild::Element(theirs)) => {
                shows_the_same(ours, theirs)
            }
            _ => false,
        }
    }
}

/// Do these two trees show the same thing? Handlers aside, as above.
fn shows_the_same(ours: &Element, theirs: &Element) -> bool {
    match (ours, theirs) {
        (Element::Text(ours), Element::Text(theirs)) => ours == theirs,
        (Element::Empty, Element::Empty) => true,
        (Element::Fragment(ours), Element::Fragment(theirs)) => {
            ours.len() == theirs.len()
                && ours
                    .iter()
                    .zip(theirs)
                    .all(|(ours, theirs)| shows_the_same(ours, theirs))
        }
        (Element::Tag(ours), Element::Tag(theirs)) => {
            ours.tag == theirs.tag
                && ours.key == theirs.key
                && ours.attributes == theirs.attributes
                && ours.children.len() == theirs.children.len()
                && ours
                    .children
                    .iter()
                    .zip(&theirs.children)
                    .all(|(ours, theirs)| shows_the_same(ours, theirs))
        }
        (
            Element::Foreign { node: ours, id: here },
            Element::Foreign {
                node: theirs,
                id: there,
            },
        ) => here == there && ours == theirs,
        _ => false,
    }
}

/// Anything that may be written between the tags of a library's
/// component.
///
/// The same marker trick as [`IntoElement`], for the same reason: a
/// blanket implementation over everything displayable would be judged
/// to overlap the one for `Element`, since Rust will not reason from
/// the absence of an implementation.
pub trait IntoForeignChild<Marker> {
    fn into_foreign_child(self) -> ForeignChild;
}

/// Markers, disjoint and never named outside this file.
pub struct ChildIsElement;
pub struct ChildIsValue;

impl IntoForeignChild<ChildIsElement> for Element {
    fn into_foreign_child(self) -> ForeignChild {
        match self {
            // A tree of nothing but text is text, which is what React
            // takes a child to be in the first place.
            Element::Text(text) => ForeignChild::Text(text),
            other => ForeignChild::Element(Box::new(other)),
        }
    }
}

impl<T: crate::value::ToValue> IntoForeignChild<ChildIsValue> for T {
    fn into_foreign_child(self) -> ForeignChild {
        ForeignChild::Value(self.to_value())
    }
}

/// Turn whatever was written between a library component's tags into a
/// child of it.
pub fn into_foreign_child<Marker, T: IntoForeignChild<Marker>>(value: T) -> ForeignChild {
    value.into_foreign_child()
}

/// A tag, its attributes, its event handlers and its children, in the
/// order they were written.
#[derive(Debug, Clone)]
pub struct TagElement {
    /// Set by a `key={…}` attribute. `None` means "compare by position".
    pub key: Option<Key>,
    pub tag: String,
    pub attributes: Vec<(String, AttributeValue)>,
    pub handlers: Vec<(String, Handler)>,
    pub children: Vec<Element>,
}

/// What an attribute is set to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeValue {
    Text(String),
    /// A bare attribute: `disabled` rather than `disabled="true"`.
    Flag(bool),
}

/// What an event handler is given.
///
/// A fixed set of fields rather than the DOM event itself, for the
/// reason the whole boundary exists: a DOM node cannot cross into Rust,
/// and what a page actually asks of an event is a short list. It was
/// `kind` alone, which meant `onInput` could not read what had been
/// typed — the one thing an `onInput` is for — and a page had to keep a
/// hidden input and a native function to get it.
///
/// Everything is read off the event where it exists and left empty
/// where it does not: a click has no `key`, a keypress has no
/// `checked`. Empty rather than an `Option` for each, because a page
/// that asks for the text of a field it knows is a field wants the
/// text, not a choice to unwrap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Event {
    /// `click`, `input`, `keydown` — the DOM's own name.
    pub kind: String,
    /// `event.target.value`: what is in the field the event came from,
    /// as text. A number input gives the digits as they were typed.
    pub value: String,
    /// `event.target.id`, so one handler can serve several elements.
    pub id: String,
    /// `event.key` for a key event: `a`, `Enter`, `Escape`, `ArrowUp`.
    pub key: String,
    /// `event.target.checked`, for a checkbox or a radio.
    pub checked: bool,
    /// Which modifiers were held down.
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    /// `Meta`: the Windows key, or `Cmd` on a Mac.
    pub meta: bool,
}

impl Event {
    /// An event of this kind and nothing else — what a test writes, and
    /// what a handler called by hand is given.
    pub fn of(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            ..Default::default()
        }
    }

    /// L'evenement, ecrit comme une valeur qui traverse.
    ///
    /// Pourquoi : une balise ordinaire ecrite dans un sous-arbre React
    /// n'est pas montee par le moteur, c'est React qui la fait, et son
    /// `onInput` revient par le chemin des props -- ou ne passent que
    /// des [`crate::Value`]. L'objet d'evenement du DOM, lui, tient des
    /// noeuds et ne traverse pas : ce sont donc ces champs-la, et pas
    /// l'objet, qui font le voyage.
    pub fn as_value(&self) -> crate::Value {
        use crate::Value;
        Value::Record(vec![
            ("kind".to_string(), Value::String(self.kind.clone())),
            ("value".to_string(), Value::String(self.value.clone())),
            ("id".to_string(), Value::String(self.id.clone())),
            ("key".to_string(), Value::String(self.key.clone())),
            ("checked".to_string(), Value::Bool(self.checked)),
            ("shift".to_string(), Value::Bool(self.shift)),
            ("control".to_string(), Value::Bool(self.control)),
            ("alt".to_string(), Value::Bool(self.alt)),
            ("meta".to_string(), Value::Bool(self.meta)),
        ])
    }

    /// Et le relire de l'autre cote.
    ///
    /// Ce qui n'est pas un evenement du tout n'est pas refuse : une
    /// bibliothèque qui appelle `onChange("tokio")` est une
    /// bibliothèque ordinaire, et refuser serait l'arreter au milieu.
    /// Le texte recu devient `value`, qui est ce qu'une closure allait
    /// en faire.
    pub fn from_value(value: &crate::Value) -> Self {
        use crate::Value;
        let text = |key: &str| value.get(key).and_then(|held| held.as_str()).unwrap_or("").to_string();
        let flag = |key: &str| value.get(key).and_then(|held| held.as_bool()).unwrap_or(false);
        match value {
            Value::Record(_) => Self {
                kind: text("kind"),
                value: text("value"),
                id: text("id"),
                key: text("key"),
                checked: flag("checked"),
                shift: flag("shift"),
                control: flag("control"),
                alt: flag("alt"),
                meta: flag("meta"),
            },
            Value::String(said) => Self {
                value: said.clone(),
                ..Default::default()
            },
            Value::Number(number) => Self {
                value: format!("{number}"),
                ..Default::default()
            },
            Value::Bool(whether) => Self {
                checked: *whether,
                value: whether.to_string(),
                ..Default::default()
            },
            _ => Self::default(),
        }
    }

    /// What was typed, as a number, when it is one.
    ///
    /// `""` is `None` and so is `"abc"`: a field being emptied is the
    /// common case, and it must not read as zero.
    pub fn number(&self) -> Option<f64> {
        self.value.trim().parse().ok()
    }
}

/// A handler written by the application, as a plain Rust closure.
///
/// Neither `Send` nor `Sync`, and that is the point: it runs on the render
/// process's UI thread and nowhere else, so it may capture an `Rc`, a
/// `Cell`, or anything else the developer already has in hand. Requiring a
/// thread bound here would make every `onClick` an argument with the
/// borrow checker over a thread the closure never reaches.
#[derive(Clone)]
pub struct Handler(Rc<dyn Fn(Event) + 'static>);

impl Handler {
    pub fn new(handler: impl Fn(Event) + 'static) -> Self {
        Self(Rc::new(handler))
    }

    pub fn call(&self, event: Event) {
        (self.0)(event)
    }
}

impl std::fmt::Debug for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Handler(..)")
    }
}

impl Element {
    /// Start building a tag. This is the call the compiler emits.
    pub fn tag(name: &str) -> TagElement {
        TagElement {
            key: None,
            tag: name.to_string(),
            attributes: Vec::new(),
            handlers: Vec::new(),
            children: Vec::new(),
        }
    }
}

impl TagElement {
    /// Identify this child across a reorder of its siblings.
    pub fn key(mut self, key: impl Into<Key>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn attribute(mut self, name: &str, value: AttributeValue) -> Self {
        self.attributes.push((name.to_string(), value));
        self
    }

    pub fn handler(mut self, name: &str, handler: Handler) -> Self {
        self.handlers.push((name.to_string(), handler));
        self
    }

    pub fn child(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }

    pub fn build(self) -> Element {
        Element::Tag(self)
    }
}

/// Anything that may be written between braces in a `.rsx` file.
///
/// `{clicks}`, `{user.name}`, `{items.len()}` — the developer writes the
/// value, never a conversion. The `Marker` parameter is what lets a single
/// function accept both an `Element` and anything merely displayable:
/// without it the blanket `T: Display` implementation would be judged to
/// overlap the specific ones, since Rust will not reason from the absence
/// of an implementation.
pub trait IntoElement<Marker> {
    fn into_element(self) -> Element;
}

/// Markers. They exist only to keep the implementations below disjoint and
/// are never named by anyone outside this file.
pub struct IsElement;
pub struct IsList;
pub struct IsOption;
pub struct IsDisplay;

impl IntoElement<IsElement> for Element {
    fn into_element(self) -> Element {
        self
    }
}

impl IntoElement<IsList> for Vec<Element> {
    fn into_element(self) -> Element {
        Element::Fragment(self)
    }
}

impl IntoElement<IsOption> for Option<Element> {
    fn into_element(self) -> Element {
        self.unwrap_or(Element::Empty)
    }
}

impl<T: Display> IntoElement<IsDisplay> for T {
    fn into_element(self) -> Element {
        Element::Text(self.to_string())
    }
}

/// Turn whatever was written between braces into a node.
pub fn into_element<Marker, T: IntoElement<Marker>>(value: T) -> Element {
    value.into_element()
}

/// What identifies a child across a reorder.
///
/// Position is not enough. A list whose rows carry state — a text field,
/// an expanded panel — reordered by position moves the state with the
/// position rather than with the row, so what was typed in the third
/// field appears in the first. A key says which row is which.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key(String);

impl<T: std::fmt::Display> From<T> for Key {
    fn from(value: T) -> Self {
        Self(value.to_string())
    }
}

impl Key {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
