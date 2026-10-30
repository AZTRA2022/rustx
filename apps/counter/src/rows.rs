//! The list of counters, whose state is identified by key — so a
//! reorder carries each row's count with it.

use rustx::state::{State, cell, with_key};
use rustx::Element;

use crate::Counter;

thread_local! {
    /// The order of the rows, so the page can ask for a rotation.
    static ORDER: std::cell::RefCell<Option<State<Vec<String>>>> =
        const { std::cell::RefCell::new(None) };
}

/// The component mounted in `#rows`.
pub fn rows() -> Element {
    let order = cell(
        rustx::state::Slot {
            file: file!(),
            line: line!(),
            column: column!(),
        },
        || vec!["a".to_string(), "b".to_string(), "c".to_string()],
    );
    ORDER.with(|slot| *slot.borrow_mut() = Some(order.clone()));

    let mut list = Element::tag("ul");
    for key in order.get() {
        list = list.child(
            Element::tag("li")
                .key(key.clone())
                .child(with_key(&key, Counter))
                .build(),
        );
    }
    list.build()
}

/// Move the last row to the front. The reconciler moves the nodes; the
/// state has to come with them.
pub fn rotate() {
    ORDER.with(|slot| {
        if let Some(order) = slot.borrow().as_ref() {
            order.update(|rows| {
                if let Some(last) = rows.pop() {
                    rows.insert(0, last);
                }
            });
        }
    });
}
