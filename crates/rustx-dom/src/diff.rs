//! Comparing the tree that is on screen with the one just rendered.
//!
//! Pure: two trees in, a list of corrections out, no DOM anywhere. That
//! is deliberate. This is the part of a framework that is easy to get
//! subtly wrong and expensive to debug through a browser, and keeping it
//! free of the DOM means it is tested in milliseconds rather than in
//! ten-second scenarios.
//!
//! A correction never carries an [`Element`]: an element holds the
//! developer's closures, so it is neither `Clone` nor comparable. A
//! correction that needs a subtree names **where to find it in the new
//! tree**, and whoever owns that tree goes and gets it.

use crate::element::{AttributeValue, Element, Key, TagElement};

/// A child path from the root of the tree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Path(pub Vec<usize>);

impl Path {
    /// The path of the tree's own root.
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// How far from the root this path is.
    pub fn depth(&self) -> usize {
        self.0.len()
    }

    fn child(&self, index: usize) -> Self {
        let mut path = self.0.clone();
        path.push(index);
        Self(path)
    }
}

/// One thing to change about what is on screen.
///
/// Not `Eq`: a prop of a node React owns may be a number, and a number
/// is not equal to itself in every case the trait promises it is.
#[derive(Debug, Clone, PartialEq)]
pub enum Patch {
    SetText {
        path: Path,
        text: String,
    },
    SetAttribute {
        path: Path,
        name: String,
        value: AttributeValue,
    },
    RemoveAttribute {
        path: Path,
        name: String,
    },
    /// Build the subtree at `source` in the new tree and put it here.
    Replace {
        path: Path,
        source: Path,
    },
    Insert {
        parent: Path,
        index: usize,
        source: Path,
    },
    Remove {
        parent: Path,
        index: usize,
    },
    Move {
        parent: Path,
        from: usize,
        to: usize,
    },
    /// Handlers are not compared — see [`diff`].
    ReplaceHandlers {
        path: Path,
        source: Path,
    },
    /// The props of a node React owns, as this render wrote them.
    ///
    /// Whole, not as a difference: React is handed the props it should
    /// have now, so a prop that went away is simply not in the list.
    ForeignProps {
        path: Path,
        props: Vec<(String, crate::Value)>,
    },
}

impl Patch {
    /// Where in the tree this correction applies.
    ///
    /// For the ones that name a parent and an index, that is the
    /// parent: the node itself is not there yet, or not there any more.
    pub fn path(&self) -> &Path {
        match self {
            Patch::SetText { path, .. }
            | Patch::SetAttribute { path, .. }
            | Patch::RemoveAttribute { path, .. }
            | Patch::Replace { path, .. }
            | Patch::ReplaceHandlers { path, .. }
            | Patch::ForeignProps { path, .. } => path,
            Patch::Insert { parent, .. } | Patch::Remove { parent, .. } => parent,
            Patch::Move { parent, .. } => parent,
        }
    }
}

/// What to change to turn `old` into `new`.
///
/// Handlers are never compared. A closure has no equality, and one from
/// the previous render has captured the handles it was given then; on a
/// node that is being patched anyway, replacing them costs a pointer
/// write and removes any chance of a stale capture surviving.
pub fn diff(old: &Element, new: &Element) -> Vec<Patch> {
    let mut patches = Vec::new();
    compare(old, new, &Path::default(), &Path::default(), &mut patches);
    patches
}

fn compare(old: &Element, new: &Element, at: &Path, source: &Path, patches: &mut Vec<Patch>) {
    match (old, new) {
        (Element::Empty, Element::Empty) => {}
        (Element::Text(before), Element::Text(after)) => {
            if before != after {
                patches.push(Patch::SetText {
                    path: at.clone(),
                    text: after.clone(),
                });
            }
        }
        (Element::Fragment(before), Element::Fragment(after)) => {
            compare_children(before, after, at, source, patches);
        }
        (Element::Tag(before), Element::Tag(after)) if before.tag == after.tag => {
            compare_tag(before, after, at, source, patches);
        }
        // Opaque on purpose. The same component is the same container,
        // so the props are handed over and nothing else is said; a
        // different component is a different thing in that container,
        // and the honest answer is to build it.
        (
            Element::Foreign {
                component: before,
                props: had,
                ..
            },
            Element::Foreign {
                component: after,
                props: wants,
                ..
            },
        ) if before == after => {
            if had != wants {
                patches.push(Patch::ForeignProps {
                    path: at.clone(),
                    props: wants.clone(),
                });
            }
        }
        // Anything else is a different kind of node, and the only honest
        // answer is to build the new one.
        _ => patches.push(Patch::Replace {
            path: at.clone(),
            source: source.clone(),
        }),
    }
}

fn compare_tag(
    old: &TagElement,
    new: &TagElement,
    at: &Path,
    source: &Path,
    patches: &mut Vec<Patch>,
) {
    for (name, value) in &new.attributes {
        let before = old
            .attributes
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, value)| value);
        if before != Some(value) {
            patches.push(Patch::SetAttribute {
                path: at.clone(),
                name: name.clone(),
                value: value.clone(),
            });
        }
    }
    for (name, _) in &old.attributes {
        if !new.attributes.iter().any(|(kept, _)| kept == name) {
            patches.push(Patch::RemoveAttribute {
                path: at.clone(),
                name: name.clone(),
            });
        }
    }

    if !old.handlers.is_empty() || !new.handlers.is_empty() {
        patches.push(Patch::ReplaceHandlers {
            path: at.clone(),
            source: source.clone(),
        });
    }

    compare_children(&old.children, &new.children, at, source, patches);
}

/// Children are matched by key as soon as **one** of them carries one,
/// and by position otherwise. Mixing the two would mean guessing, and a
/// list that is half keyed is a mistake worth leaving visible rather
/// than papering over.
fn compare_children(
    old: &[Element],
    new: &[Element],
    at: &Path,
    source: &Path,
    patches: &mut Vec<Patch>,
) {
    let keyed = old.iter().chain(new).any(|child| key_of(child).is_some());
    if keyed {
        compare_keyed(old, new, at, source, patches);
    } else {
        compare_by_position(old, new, at, source, patches);
    }
}

fn compare_by_position(
    old: &[Element],
    new: &[Element],
    at: &Path,
    source: &Path,
    patches: &mut Vec<Patch>,
) {
    for (index, (before, after)) in old.iter().zip(new).enumerate() {
        compare(
            before,
            after,
            &at.child(index),
            &source.child(index),
            patches,
        );
    }
    for index in old.len()..new.len() {
        patches.push(Patch::Insert {
            parent: at.clone(),
            index,
            source: source.child(index),
        });
    }
    // Backwards, so each index is still the index it was.
    for index in (new.len()..old.len()).rev() {
        patches.push(Patch::Remove {
            parent: at.clone(),
            index,
        });
    }
}

/// Match children by key, falling back to position for the ones that
/// carry none.
///
/// A list is rarely all keys. `<ul>Items:<li key={id}>…</li></ul>` has
/// a heading among its children, and a key read off real data is not
/// always unique. Neither may drop a child, and neither may be allowed
/// to address a child that is not there: every index below is an index
/// into a list this function is also rewriting, which is exactly where
/// an off-by-one turns into a lost node or a panic.
fn compare_keyed(
    old: &[Element],
    new: &[Element],
    at: &Path,
    source: &Path,
    patches: &mut Vec<Patch>,
) {
    // A key that appears twice identifies nothing, so the second one is
    // treated as unkeyed rather than silently shadowing the first.
    let old_keys = unique_keys(old);
    let new_keys = unique_keys(new);

    // Each new child's old counterpart: by key where there is one, and
    // otherwise by order of appearance among the unkeyed.
    let mut spare: Vec<usize> = (0..old.len()).filter(|index| old_keys[*index].is_none()).collect();
    let mut taken = vec![false; old.len()];
    let mut partner: Vec<Option<usize>> = Vec::with_capacity(new.len());

    for key in &new_keys {
        let found = match key {
            Some(key) => old_keys
                .iter()
                .position(|existing| existing.as_ref() == Some(key))
                .filter(|index| !taken[*index]),
            None => (!spare.is_empty()).then(|| spare.remove(0)),
        };
        if let Some(index) = found {
            taken[index] = true;
        }
        partner.push(found);
    }

    // What nobody claimed goes, highest index first so the rest keep
    // the numbers this loop has already used.
    for index in (0..old.len()).rev() {
        if !taken[index] {
            patches.push(Patch::Remove {
                parent: at.clone(),
                index,
            });
        }
    }

    // `order` tracks the children as they now stand, so that every
    // `from` below is where the node actually is at that moment.
    let mut order: Vec<Option<usize>> = (0..old.len())
        .filter(|index| taken[*index])
        .map(Some)
        .collect();

    for (target, slot) in partner.iter().enumerate() {
        match slot.and_then(|old_index| order.iter().position(|held| *held == Some(old_index))) {
            Some(from) if from == target => {}
            Some(from) => {
                patches.push(Patch::Move {
                    parent: at.clone(),
                    from,
                    to: target,
                });
                let moved = order.remove(from);
                order.insert(target.min(order.len()), moved);
            }
            None => {
                let index = target.min(order.len());
                patches.push(Patch::Insert {
                    parent: at.clone(),
                    index,
                    source: source.child(target),
                });
                order.insert(index, None);
            }
        }
    }

    // Finally, what each surviving child became.
    for (target, slot) in partner.iter().enumerate() {
        let Some(before) = slot else { continue };
        compare(
            &old[*before],
            &new[target],
            &at.child(target),
            &source.child(target),
            patches,
        );
    }
}

/// The children's keys, with every repeat of a key demoted to `None`.
fn unique_keys(children: &[Element]) -> Vec<Option<Key>> {
    let mut seen: Vec<Key> = Vec::new();
    children
        .iter()
        .map(|child| match key_of(child) {
            Some(key) if seen.contains(&key) => None,
            Some(key) => {
                seen.push(key.clone());
                Some(key)
            }
            None => None,
        })
        .collect()
}

fn key_of(element: &Element) -> Option<Key> {
    match element {
        Element::Tag(tag) => tag.key.clone(),
        _ => None,
    }
}
