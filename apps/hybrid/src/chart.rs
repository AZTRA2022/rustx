//! What this application is about: one value, two owners.
//!
//! `app/page.rsx` hands `<js::Chart points={…} />` to JavaScript and
//! leaves `#panel-host` for JavaScript to fill — and what JavaScript
//! puts in it is [`counter`], a rustx component asked for by name. Both
//! read the same store, which is how one frame is seen to have drawn
//! both sides.

use rustx::store::store;
use rustx::{AttributeValue, Element};

/// How many points the chart is showing.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Points(pub u32);

/// Add a point. Both halves of the page are watching this one value.
pub fn add_point() {
    let points = store::<Points>();
    let now = points.get().0;
    points.set(Points(now + 1));
}

/// Leave the page, which takes the JavaScript component's container with
/// it — and JavaScript is told before it goes.
pub fn leave() {
    rustx::router::navigate("/gone".to_string());
}

/// The component JavaScript's panel asks rustx for. It shows the same
/// number the chart does.
pub fn counter() -> Element {
    Element::tag("span")
        .attribute("id", AttributeValue::Text("counter-in-panel".into()))
        .child(Element::Text(store::<Points>().get().0.to_string()))
        .build()
}
