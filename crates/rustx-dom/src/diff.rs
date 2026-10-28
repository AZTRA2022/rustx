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
    fn child(&self, index: usize) -> Self {
        let mut path = self.0.clone();
        path.push(index);
        Self(path)
    }
}

/// One thing to change about what is on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
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

fn compare_keyed(
    old: &[Element],
    new: &[Element],
    at: &Path,
    source: &Path,
    patches: &mut Vec<Patch>,
) {
    let old_keys: Vec<Option<Key>> = old.iter().map(key_of).collect();
    let new_keys: Vec<Option<Key>> = new.iter().map(key_of).collect();

    // What goes, highest index first so the rest keep their numbers.
    let mut living: Vec<usize> = Vec::new();
    for (index, key) in old_keys.iter().enumerate() {
        if key.is_some() && new_keys.contains(key) {
            living.push(index);
        }
    }
    for index in (0..old.len()).rev() {
        if !living.contains(&index) {
            patches.push(Patch::Remove {
                parent: at.clone(),
                index,
            });
        }
    }

    // The survivors, in the order they are in now.
    let mut order: Vec<Key> = living
        .iter()
        .filter_map(|index| old_keys[*index].clone())
        .collect();

    for (target, key) in new_keys.iter().enumerate() {
        let Some(key) = key else { continue };
        match order.iter().position(|existing| existing == key) {
            Some(from) if from == target => {}
            Some(from) => {
                patches.push(Patch::Move {
                    parent: at.clone(),
                    from,
                    to: target,
                });
                let moved = order.remove(from);
                order.insert(target, moved);
            }
            None => {
                patches.push(Patch::Insert {
                    parent: at.clone(),
                    index: target,
                    source: source.child(target),
                });
                order.insert(target, key.clone());
            }
        }
    }

    // Finally, what each surviving child became.
    for (target, key) in new_keys.iter().enumerate() {
        let Some(key) = key else { continue };
        let Some(before) = old_keys.iter().position(|existing| existing.as_ref() == Some(key))
        else {
            continue;
        };
        compare(
            &old[before],
            &new[target],
            &at.child(target),
            &source.child(target),
            patches,
        );
    }
}

fn key_of(element: &Element) -> Option<Key> {
    match element {
        Element::Tag(tag) => tag.key.clone(),
        _ => None,
    }
}
