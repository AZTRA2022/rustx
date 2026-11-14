# rustx

> **This is the reference version, and the one that is kept current.**
> The original French page is kept as it was written, at
> [`README.fr.md`](README.fr.md). Where the two disagree, this one is
> right: the French page is no longer updated and may name a version or
> an API that has moved.

Native software, written in Rust, drawn by a browser engine.

Components are written in `.rsx` files — JSX's syntax, Rust's language —
and a `let mut` in one of them is state: no annotation, no macro, no
`store::`. npm libraries (React, Recharts, Tailwind, shadcn) are used
from the `.rsx` itself. A project's `main.rs` is four lines, because the
ceremony is the framework's job.

Rust and React live in **the same process**: an `onClick` is a function
call, not a serialised message, and props cross the boundary as V8
values.

> **State of the work.** The workspace is `0.3.0` and the API can still
> move. Windows is written and measured. macOS and Linux compile, and
> each system call that is not written there refuses by naming the
> platform and the file that would implement it, rather than doing
> nothing quietly. Nothing is published to crates.io yet.

In order:
[install](#install) · [a project](#a-project) ·
[the .rsx file](#the-rsx-file) · [state](#state) ·
[routes](#routes) · [async](#async) · [worker threads](#worker-threads) ·
[JavaScript libraries](#javascript-libraries) ·
[the window](#the-window) ·
[what the web cannot do](#what-the-web-cannot-do) ·
[the commands](#the-commands) · [the editor](#the-editor) ·
[when it does not work](#when-it-does-not-work) ·
[the packages](#the-packages) · [in this repository](#in-this-repository)

The long form of every section here is in [`site/content/docs`](site/content/docs),
which is the documentation site's own source: 80 pages, each with
examples that compile.

---

## Install

```sh
cargo install rustx-cli      # rustx new, dev, build, run
```

Rust 1.88 or newer. Nothing else: CEF — the browser engine — arrives
with the `cef` crate on the first build, and an application that uses no
npm library needs neither Node nor npm.

For JavaScript libraries, Node and npm, like any project that uses them.

## A project

```sh
rustx new my-app
cd my-app
rustx dev
```

Five files, and a window:

```text
my-app/
  Cargo.toml        rustx = { package = "rustx-app", version = "0.3" }
  src/main.rs       the window, in four lines
  app/page.rsx      the `/` page
  assets/           what the page loads (an icon, an image, an index.html)
  .gitignore        /target
```

The package is called `rustx-app` on crates.io, because `rustx` there
belongs to another project; the **library** is called `rustx`, so all
the code is written `rustx::`:

```toml
[dependencies]
rustx = { package = "rustx-app", version = "0.3" }
```

`main.rs`, in full:

```rust
use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("My app").size(900, 640))?;

    app.run()
}
```

`#[rustx::main]` does the rest: it compiles `app/`'s `.rsx` files to
Rust, writes the route table, builds the JavaScript bundle and the
stylesheet if there is a `package.json`, finds or writes
`assets/index.html`, and gets Chromium's subprocesses out before they
touch anything. None of those lines varies from one application to the
next, so none of them is yours to write.

There is **no `build.rs`**, and nothing to configure.

## The `.rsx` file

An `.rsx` file is Rust, with markup in the `return`s. A component is a
function that gives back an `Element`:

```rust
fn Page() -> Element {
    return <main>
        <h1>Hello.</h1>
    </main>;
}
```

Between two tags, text is text; between braces it is Rust, and the value
is shown:

```rust
fn Page() -> Element {
    let name = "world";
    let times = 3;

    return <p>Hello {name}, {times} times, {times * 2} in all.</p>;
}
```

Anything that knows how to become an `Element` or text can go there:
numbers, strings, booleans, `Option`, `Vec<Element>`.

Comments are Rust's, including **inside** markup — `//` starts a comment
when it starts a line, so `see https://example.org` stays a sentence.
`{/* JSX's own form */}` works too.

### Attributes

Tags take **HTML's and JSX's props**, and the compiler refuses what does
not exist by suggesting the nearest name:

```rust
return <button
    id="go"
    className="primary large"
    disabled={busy}
    aria-label="Start the render"
    data-kind="action"
    onClick={move |_| start()}
>Start</button>;
```

- `className` and not `class`, `onClick` and not `onclick`: JSX's
  spelling;
- `aria-*` and `data-*` are taken as written, hyphens included;
- a boolean attribute takes an expression (`disabled={true}`);
- a handler takes a closure: `onClick={move |_| …}`,
  `onInput={move |event| …}`.

What the event carries — a short, fixed list, because a DOM node does
not cross:

| | |
|---|---|
| `event.kind` | `click`, `input`, `keydown`: the DOM's name |
| `event.value` | `target.value`, as text |
| `event.number()` | the same, as a number, when it is one |
| `event.id` | `target.id`, so one handler can serve several elements |
| `event.key` | `a`, `Enter`, `Escape`, `ArrowUp` |
| `event.checked` | for a checkbox |
| `event.shift` `event.control` `event.alt` `event.meta` | the modifiers |

`style` is written as an object, as in JSX, with `camelCase` keys:

```rust
return <div style={{
    display: "grid",
    placeItems: "center",
    height: "100vh",
    backgroundColor: "#0b0b10",
}}>…</div>;
```

### Components, and their props

A component of the project is called **like a Rust function**, inside
braces. Its props are its arguments, with Rust's types. An uppercase
*tag* is reserved for the components of JavaScript libraries.

```rust
fn Page() -> Element {
    return <main>
        {Card("Rust".to_string(), 3)}
        {Card("React".to_string(), 1)}
    </main>;
}

fn Card(title: String, times: u32) -> Element {
    return <article className="card">
        <h2>{title}</h2>
        <p>{times} times</p>
    </article>;
}
```

### Lists and conditions

Ordinary Rust: `map` and `collect` for a list, `if` or `match` for a
choice. A `for` loop is a statement in Rust, so it has no value and
cannot sit inside `{…}`.

```rust
return <ul>
    {names.iter().map(|name| <li key={name.clone()}>{Counter(name.clone())}</li>).collect::<Vec<_>>()}
</ul>;

return <div>{if busy { Waiting() } else { Result(value) }}</div>;
```

`key` is what state follows, exactly as React asks: `order.rotate_right(1)`
moves the keys and the state each one holds together.

> A loop that does not end, written **before** the `return`, blocks the
> frame: rendering is synchronous, and a page that does not return its
> `Element` draws nothing. Work that takes time goes in a
> [`task`](#worker-threads) or an [`async` block](#async).

## State

A `let mut`, and **where it is written says what its scope is.** There
is nothing else to learn.

```rust
let mut points = 0;              // at the top of the file: shared

fn Page() -> Element {
    let mut open = false;        // inside a component: private

    return <div>
        <p>{points} points</p>
        <button onClick={move |_| points += 1}>one more</button>
        <button onClick={move |_| open = !open}>{open}</button>
    </div>;
}
```

- **at the top of the file**: shared by the application. It carries a
  name, so it survives a `rustx dev` reload;
- **inside a component**: one cell per place in the tree. Two
  `{Card(…)}` have one each, and leaving the screen frees them.

Reading is reading. Writing marks the screen for the next frame: ten
thousand writes in one frame draw **once**. There is no debounce to tune
and no rate limit to learn.

State is not written **during** a render: a component that is working
out its screen does not change it at the same time. The write goes in a
handler, in an `effect`, or in a `task`, and the compiler says so with
those three words when you forget.

A page value is taken by copy when it is read, so a write through a
mutable borrow — `rows.get_mut(i)`, `rows.iter_mut()` — is refused
rather than silently changing a copy: take the copy by name, change it,
and put it back.

### `keep`, `effect`, `task`

```rust
fn Panel() -> Element {
    // Built once, not on every render.
    let client = keep(Client::connect());

    // Replayed when what it watches changes. `()`: once.
    effect(filter.clone(), move || reload(&filter));

    // A thread that stops when this component leaves the tree.
    task(move |alive| {
        while alive.get() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = alive.post(|| ticks += 1);
        }
    });

    return <p>{ticks}</p>;
}
```

## Routes

Routes are the `app/` directory:

```text
app/page.rsx                     →  /
app/settings/page.rsx            →  /settings
app/device/[id]/page.rsx         →  /device/<something>
app/files/[...path]/page.rsx     →  /files/<everything else>
app/help/[[...path]]/page.rsx    →  /help, and /help/<everything else>
app/(shop)/cart/page.rsx         →  /cart  (the group is not in the URL)
app/layout.rsx                   →  a frame around everything below it
app/repos/loading.rsx            →  what is shown while the page has nothing
app/repos/error.rsx              →  what is shown if the page falls over
```

A bracketed segment arrives as an argument of the component, in the
order it is written. When several routes describe the same path, the
most specific wins: a static segment beats a named one, which beats a
catch-all.

```rust
rustx::router::navigate("/settings");
rustx::router::replace("/thanks");     // going back will not return here
rustx::router::back();                 // false if there is no back
rustx::router::forward();
rustx::router::can_go_back();          // what a button asks before greying out
let here = rustx::router::current();   // with the query string
let route = rustx::router::route();    // the path alone
let term = rustx::router::query("q");  // "" if nobody wrote it
rustx::router::navigate(rustx::router::with_query("sort", Some("stars")));
```

History is a list and a finger on it: going back moves the finger and
keeps what is ahead, going somewhere else erases that future. That is a
browser's rule.

A **layout** takes the parameters of its own directory, and only those:

```rust
// app/repos/[owner]/layout.rsx
fn Layout(children: Element, owner: String) -> Element {
    return <section><h1>{owner}</h1>{children}</section>;
}
```

A layout that still surrounds the new screen keeps its cells; the screen
being left is freed, and the threads it held stop.

### Waiting, and falling over

A page waiting on an `.await` draws nothing until the answer comes,
which is correct and is also a blank screen. The nearest `loading.rsx`
goes there, **inside the same layouts**: the top bar stays, and only the
middle says "one moment".

If the page falls over — an `unwrap` on nothing, an index past the end —
the nearest `error.rsx` takes its place, with what the panic said,
instead of the window going away:

```rust
// app/repos/error.rsx
fn Error(why: String) -> Element {
    return <p>that did not work: {why}</p>;
}
```

With no `error.rsx` above it, a page that falls over behaves as it did
before: nothing is caught, because catching everywhere would turn a bug
into a screen nobody asked for.

Without either, the engine draws its own: **"This screen does not
exist"** for a path nobody declared, and **"This screen could not be
drawn"** with the panic's own words for a page that fell over. Both
carry a named id (`rustx-no-such-screen`, `rustx-screen-failed`) so a
test can tell them from a page that happens to say the same thing.

`Config` says where the application starts and what a route nobody
declared leads to:

```rust
Config::new("My app")
    .start("/settings")          // `/` by default
    .not_found(NotFound)         // otherwise the engine's own screen
```

## Async

An `async` block written as a statement of a component is work that runs
beside the render, carried by a tokio runtime on its own threads. There
is nothing to wrap it in:

```rust
let mut lines = Vec::<String>::new();

fn Panel() -> Element {
    async move {
        let answer = load().await;
        lines.push(answer);
    };

    return <ul>{lines.len()} lines</ul>;
}
```

A block that is a **value** is left alone: `let held = async { … };` is a
future the component keeps. `task(async { … })` means the same thing, for
whoever prefers to name it.

A component itself is never `async`: `async fn Page()` is refused in so
many words, and the refusal names `task(…)`. A component is asked for the
screen as it is **now**; if it could wait, the frame would wait with it.

`main` can wait, though:

```rust
#[rustx::main]
async fn main() -> Result<(), rustx::Error> {
    let settings = tokio::fs::read_to_string("settings.toml").await?;
    let app = App::new(Config::new(&title_of(&settings)).size(900, 640))?;
    app.run()
}
```

And a component can await a value without blocking the render:

```rust
fn Settings() -> Element {
    let text = tokio::fs::read_to_string("settings.toml").await.unwrap_or_default();
    return <pre>{text}</pre>;
}
```

The first frame asks and draws nothing; the frame after the answer
arrives draws it. The question is asked once, and the rest of the page is
drawn in the meantime.

Reading and writing are deliberately not symmetric. A **read** of a page
value is direct, wherever it is written. A **write** goes back through the
frame — which is what marks the page, what makes a thousand writes cost
one render, and what refuses the write of a component that has already
gone. It follows that *a future that writes and then reads sees the old
value until the frame that applies the write.*

Two limits: the value must be `Send + Sync`, and it must be a value **of
the page**, declared at the top of the file — a `let mut` inside a
component belongs to the instance that is drawing, so it is refused in a
block, and the refusal says where to declare it. A read **after** an
`.await` is refused for the same reason.

`tokio` comes from the facade, so an application names it neither in its
`Cargo.toml` nor in its `use` lines, and writes it by the name everyone
gives it. It is tokio in full, with `full`: `tokio::fs`, `::net`, `::io`,
`::sync`, `::process`, `select!`, `spawn`. The runtime starts at the
**first** future of the process and never before.

## Worker threads

A thread never touches state. It posts work, and the frame drains it:

```rust
task(move |alive| {
    let stream = open_the_camera();
    while alive.get() {
        let frame = stream.read();
        if alive.post(move || latest = frame).is_err() {
            break;          // the window has gone, or this component
        }
    }
});
```

- `alive.get()`: is this component still there;
- `alive.post(work)`: to run on the interface thread, at the start of the
  next frame;
- `Err(PostError::WindowClosed)` / `Err(PostError::Cancelled)`: stop.

Outside a component, `rustx::poster()` gives the same token, and
`rustx::task::spawn(work)` a thread whose `Task` you keep.

## JavaScript libraries

A `package.json` and an `npm install`, and that is all: the framework
reads `dependencies`, writes the entry point, builds the bundle with
esbuild, and an uppercase tag is resolved at runtime.

```sh
npm install react react-dom recharts
npx shadcn add button card dialog
```

```rust
return <Card>
    <CardHeader><CardTitle>Sales</CardTitle></CardHeader>
    <CardContent>
        <LineChart width={520} height={240} data={points}>
            <XAxis dataKey="at" />
            <Line dataKey="value" stroke="#8b5cf6" />
        </LineChart>
    </CardContent>
</Card>;
```

**No prefix to write.** The application's own components come before the
libraries they are built from: `<Dialog>` is the one in
`components/ui/dialog.tsx`, not the `@radix-ui/react-dialog` primitive
underneath it. To ask for the other one explicitly:
`<radixUiReactDialog::Dialog>`. The name before `::` is the package's,
with what a line of Rust will not take removed.

`npx shadcn add` works. `npx shadcn init` does **not**: it looks for a
framework it knows — Next, Vite, Remix — and stops. What it would have
written is two files, written by hand once: `components.json` and
`tsconfig.json`. Both are given in full in
[`site/content/docs/javascript-libraries/shadcn.mdx`](site/content/docs/javascript-libraries/shadcn.mdx).

A React component of your own — the one that calls a hook, which an
`.rsx` page cannot do — goes in `components/`, and its `export default`
is a tag under its file's name:

```tsx
// components/Editor.tsx
export default function Editor({ content }: { content?: string }) { … }
```

```rust
return <Editor content="<p>Write here.</p>" />;
```

Props cross as V8 values, in the same process: no JSON, no serialised
bridge. `#[derive(ToValue)]` on a struct and it passes; for an object
written on the spot, `record([("at", 1.0.to_value())])`.

Tailwind works, in version 3 as in 4, and the framework reads which one
`node_modules` holds. In 4 the command line lives in its own package, so
both are needed: `npm install tailwindcss @tailwindcss/cli`. Classes are
looked for in `app/**/*.rsx`, `src/**/*.rs`, `components/**` and
`lib/**`. A stylesheet that declares `--background` does **not** thereby
give `bg-background`: that bridge is `@theme inline`, and a sheet copied
by hand is where it goes missing.

The other direction — a rustx component mounted by React — is
`Config::new("My app").mount("panel", Panel)`, and `<div id="panel" />`
in the JSX.

## The window

`Config` carries what a window is: `.size(w, h)`, `.icon(path)`,
`.version(…)`, `.description(…)`, `.publisher(…)`, `.index(path)`,
`.host(id)`, `.start(route)`, `.not_found(component)`,
`.mount(id, component)`, `.on_context_created(…)`, `.frameless()`, and
`.without_scrollbars()`.

`Config::frameless()` removes the title bar and the three buttons; the
edges you grab to resize stay, and so does the drop shadow. What the
frame did, the page draws, and `rustx::window` gives it: `drag()`,
`minimize()`, `maximize()`, `restore()`, `fullscreen()`,
`toggle_maximize()`, `toggle_fullscreen()`, `on_top(bool)`,
`frameless()` / `framed()`, `shut()`.

```rust
<header className="flex items-center gap-2 border-b px-3 py-2">
    <span className="font-semibold">My software</span>
    <span className="h-6 flex-1" onMouseDown={|_| { rustx::window::drag(); }} />
    <button onClick={|_| { rustx::window::minimize(); }}>–</button>
    <button onClick={|_| { rustx::window::toggle_maximize(); }}>□</button>
    <button onClick={|_| { rustx::window::shut(); }}>×</button>
</header>
```

The maximise button of a bar of your own **toggles**: telling an
already-maximised window to maximise does nothing, and a button that
works once is what the other call gives you. The drag handle sits on
empty space rather than on the whole bar, because a rustx event does not
yet say what was pressed, so a draggable bar would swallow its own
buttons' clicks.

A page can open a **second window**, on the route it chooses:

```rust
let _ = rustx::window::open(
    rustx::window::What::new("settings")   // its name, to close it again
        .titled("Settings")
        .size(520, 400)
        .at("/settings"),                  // the route it opens on
);
```

Opening the same name twice brings that window forward instead of making
a second one. Closing a window does not end the application while one is
left. Each window has its own render process, so its own state: what they
share, they share by a message.

## What the web cannot do

A page cannot read the clipboard when it likes, or open a dialog that
gives back a path, or put an icon next to the clock. That is what
`rustx-os` does, and it is written in a page like anything else.

Every module answers **`supported()`** before being asked for anything,
so a missing capability is a button you do not draw rather than an error
to read. And everything registered with the system is a **token**:
dropped, it is withdrawn — a dead icon in the tray, or a `Ctrl+Shift+R`
no application answers, are defects a user can only fix by rebooting.

```rust
use rustx::{autostart, clipboard, devices, dialog, dropped, instance, notify, shortcut, terminal, tray};

clipboard::set_text("copied from Rust")?;
let pasted = clipboard::text()?;
let files = dialog::open(dialog::Ask::new("Choose an image").filter("Images", ["png", "jpg"]).many())?;
let icon = tray::Icon::new(Path::new("assets/icon.ico"), "My software")?;
notify::show(notify::What::new("Done", "The render has finished."))?;
let capture = shortcut::Shortcut::listen("Ctrl+Shift+R")?;
if capture.pressed() > 0 { start(); }
dropped::on_files(move |paths| { for path in paths { open(&path); } });
let Ok(_only) = instance::Only::one("my-software") else { … };
autostart::set(true, "my-software")?;
for device in devices::devices() { … }
```

And twelve more capabilities, each under its own short name on the
facade:

| | |
|---|---|
| `rustx::about` | the system's own About panel |
| `rustx::attention` | flash the taskbar, a progress bar on the icon, a badge |
| `rustx::audio` | the endpoints, the volume, mute |
| `rustx::bluetooth` | what is paired |
| `rustx::capture` | the desktop, an area, a window, as pixels |
| `rustx::color` | the system colour picker |
| `rustx::net` | online, metered, what kind of connection |
| `rustx::print` | the print dialog, silent printing, the printers |
| `rustx::screen` | the screens, the primary one, the one holding a point |
| `rustx::secret` | the keychain: `store`, `fetch`, `forget` |
| `rustx::shell` | reveal a path, the recent list, file associations |
| `rustx::system` | theme, power, idle time, locale, timezone, scale |
| `rustx::terminal` | `shell()`, `run(script, at)`, `open(dir)` — and it blocks, so it belongs inside a `task(…)` |

`rustx::os::` reaches all of them under their full module path as well,
and holds the three that have no short name: `os::window` (because
`rustx::window` is what the page commands), `os::blocking` and
`os::patience`.

One directory per system inside — `windows/`, `linux/`, the rest — and a
capability is one file per system with the same function names. Windows
is written. Elsewhere, the refusal names the platform **and the file to
write**, and the tests compile all three on any machine.

What needs an eye is looked at by hand:

```sh
cargo run -p rustx-os --example choose_a_file
cargo run -p rustx-os --example tray_icon
cargo run -p rustx-os --example a_global_shortcut
cargo run -p rustx-os --example what_is_plugged_in
```

## The commands

| | |
|---|---|
| `rustx new <name>` | writes a project that compiles first time |
| `rustx dev` | watches the files, rebuilds, brings the window back |
| `rustx build` | builds, and gives the `.exe` its name and icon |
| `rustx build --release` | the version to ship |
| `rustx run` | builds and runs once, without watching |
| `rustx --help` | the commands, and what a reload keeps |

`cargo build` still works: the binary runs, it only looks anonymous in
the explorer — `rustx build` adds the icon and the version to the file's
properties.

`rustx dev` **restarts** the application; it does not hot-swap native
code and does not pretend to. Across the restart: the shared state (the
`let mut` at the top of an `.rsx`, which carries a name) and the page
that was open. Not across it: a component's cells, whose identity is a
place in code that has just changed, and any value that cannot be written
on one line. A value whose *type* changed is dropped rather than forced
back in.

What runs during a `rustx dev` is a **copy** of the binary, in
`target/rustx-dev/`: a running `.exe` is held by Windows, and the next
build has to be able to overwrite the one cargo produced.

## The editor

The VS Code extension is in [`editors/vscode`](editors/vscode):

```sh
rustup component add rust-analyzer
cargo install rustx-lsp
code --install-extension editors/vscode/rustx-framework-extension-0.3.0.vsix
```

What it gives: `.rsx` highlighting; the compiler's errors underlined at
**their real coordinates** in the `.rsx` and not in the generated Rust;
completion of HTML and SVG tags, attributes, events, `style={{ … }}`
keys and the file's own components; completion of **the components of
the installed libraries**, read from `node_modules` and
`components/ui`, with the library's name beside each; completion of
**what those tags take** (`variant`, `size`, `asChild` on a `<Button `),
read from the package's `.d.ts` or from the `components/ui/*.tsx` that
`shadcn add` wrote; **tag closing**, where typing the `>` of `<Card>`
writes `</Card>` behind the cursor and `Vec<u32>`, `a < b >` and `=>` are
left alone (`rustx.closeTags` turns it off); and inside a `{ … }`,
`rust-analyzer` answers from behind the server.

To see what the editor will offer in a given application:

```sh
cargo run -p rustx-lsp --example what_is_installed -- <the app's directory>
```

## When it does not work

**"This screen does not exist"** — the route asked for is declared
nowhere. Check the path given to `navigate`, the directory's name under
`app/`, and `Config`'s `.start(…)`.

**"the page asked for `Button`, and nothing has it"** — the name is
neither a function of the page nor an export of an installed library.
The message lists the libraries it found. Usually a forgotten `npm
install`, or a component of the page called as a tag (`<Card />` instead
of `{Card()}`).

**"this application declares JavaScript libraries … and has no
node_modules"** — `npm install`, in the application's directory.

**"this application installed tailwindcss and nothing that can run it"**
— Tailwind 4 keeps its command line in `@tailwindcss/cli`.

**The window is white and the log repeats "Failed to create shared
context"** — the machine has no usable 3D driver (a virtual machine, a
remote desktop). `RUSTX_SOFTWARE=1` composites on the processor: slower,
and it always works. The same holds for the test suite, half of which
waits for a frame: `RUSTX_SOFTWARE=1 cargo test --workspace`.

**A message between the window and the page does not arrive** —
`RUSTX_TRACE=1` traces both ends: what leaves, what arrives, and what
nobody is listening to.

**"a window can only be opened from a page"** — `window::open` was
called from `main` or from a thread; it is called from the page, which is
the only place that has the frame to carry the request.

**The compiler talks about an `onClick`, an `effect` or a `task`** —
state was written during a render. The write goes in one of the three.

## The packages

Eight crates, none of them published yet.

| | |
|---|---|
| `rustx-app` | the facade: `App`, `Config`, `#[rustx::main]` — written `rustx::` |
| `rustx-cli` | `rustx new`, `dev`, `build`, `run` |
| `rustx-lsp` | what an editor asks about an `.rsx` |
| `rustx-compiler` | `.rsx` → `.rs`, and the position map |
| `rustx-dom` | the tree, the state, the diff, the routes |
| `rustx-engine` | the CEF host, the frame loop, the router |
| `rustx-os` | the clipboard, the dialogs, the tray, the rest of the system |
| `rustx-macro` | `#[rustx::main]`, `#[derive(ToValue)]` |

An application names one of them: `rustx-app`. The others come with it.
An application that outgrows the facade can call `rustx_engine`
directly: nothing is hidden, and every line the attribute adds is code
you could have written by hand.

## In this repository

| | |
|---|---|
| [`crates/`](crates) | the eight packages, each with its own README |
| [`apps/`](apps) | fourteen applications, which are what the plans are proved against |
| [`site/`](site) | the documentation site, and the source of every page linked above |
| [`editors/vscode`](editors/vscode) | the extension |
| [`scripts/gates.ps1`](scripts/gates.ps1) | the quality gates, nine of them |
| [`docs/stability/`](docs/stability) | what each subsystem was measured at, and what is still open |
| [`BUG.md`](BUG.md) | what is known broken |
| [`STABILITY.md`](STABILITY.md) | the brief the current work answers to |

The gates run with `powershell -File scripts/gates.ps1`, and
`-List` prints them. `.github/workflows/ci.yml` runs the same ones, plus
the two Node trees and a `portable` job that builds the parts that do not
need CEF on Linux and macOS.

## The licence

MIT or Apache-2.0, at your choice — [`LICENSE-MIT`](LICENSE-MIT) and
[`LICENSE-APACHE`](LICENSE-APACHE).
