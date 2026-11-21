# rustx-dom

**The element tree a rustx component returns, its state, and its
routes.** An internal crate of
[rustx](https://github.com/AZTRA2022/rustx).

> **You probably want the `rustx` crate instead** — the package
> `rustx-app`, which re-exports everything here under `rustx::`. An
> application depends on one crate, and this is one of the five it is
> made of.
>
> ```toml
> rustx = { package = "rustx-app", version = "0.3" }
> ```

## What it is

Deliberately the smallest thing that can describe a user interface: a
tag, a piece of text, a list of siblings, or nothing.

```rust,ignore
pub enum Element {
    Tag(TagElement),
    Text(String),
    List(Vec<Element>),
    Foreign(ForeignNode),   // a React component, mounted by the engine
    Nothing,
}
```

It sits in a crate of its own for one reason: the compiler emits the
calls that build this tree, and the engine walks the tree to touch the
real DOM. **The compiler must not depend on the web view, and the engine must not
depend on a parser.** This is what both of them can depend on instead.

## What is in it

| Module | What it holds |
| --- | --- |
| `element` | `Element`, `TagElement`, `AttributeValue`, `Event`, `Callback` |
| `diff` | the comparison of two trees, which is what the engine sends |
| `state` | a component's own cells, keyed by where they are in the code |
| `store` | `store()` for a component's state, `shared()` for a page's |
| `route` | the route table and which route is current |
| `value` | `Value`, the one type that crosses into JavaScript |
| `carried` | the state that survives a `rustx dev` relaunch |

```rust,ignore
use rustx_dom::store;

// What `let mut clicks = 0;` at the top of a page compiles to.
let clicks = store::shared("clicks", || 0_i32);
clicks.set(clicks.get() + 1);
```

## Dependencies

**None.** Not one crate, on any platform. That is on purpose: everything
else in the workspace depends on this, so anything this depended on
would be in every rustx application.

Rust 1.88 or newer, edition 2024. Licensed under MIT or Apache-2.0, at
your option.
