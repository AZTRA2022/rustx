//! The one crate an application depends on.
//!
//! A `main.rs` is a window's title and its size:
//!
//! ```ignore
//! use rustx::{App, Config};
//!
//! #[rustx::main]
//! fn main() -> Result<(), rustx::Error> {
//!     let app = App::new(Config::new("Console").size(800, 600))?;
//!     app.run()
//! }
//! ```
//!
//! Everything that used to be in that file is here instead: leaving a
//! sub-process before it touches anything, finding `assets/index.html`,
//! compiling the pages of `app/` into the binary, and handing the
//! router the table the build script wrote. None of it varies between
//! applications, so none of it is an application's to write.
//!
//! Nothing here hides anything: every line the attribute adds is
//! ordinary code that could have been written by hand, which is what
//! lets an application that outgrows this reach for `rustx_engine`
//! directly and keep going.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;

pub use rustx_macros::main;

pub use rustx_dom::{
    AttributeValue, Element, Handler, IntoElement, Key, TagElement, Value, into_element, route,
    state, store,
};
pub use rustx_engine::{
    Document, DomError, NativeError, Node, PostError, Poster, foreign, mount, mount_at_id,
    on_context_created, on_frame, poster, register_native, render, router, task,
};

/// What can go wrong while an application starts.
pub type Error = rustx_engine::EngineError;

thread_local! {
    /// The directory of the crate being run, as `env!` saw it there.
    static MANIFEST_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    /// The route table the build script wrote.
    static ROUTES: Cell<Option<&'static [route::Route]>> = const { Cell::new(None) };
}

/// Said by `#[rustx::main]`, which is where `env!` reads the
/// application's own crate rather than this one.
#[doc(hidden)]
pub fn install_manifest_dir(directory: &str) {
    MANIFEST_DIR.with(|held| *held.borrow_mut() = Some(PathBuf::from(directory)));
}

/// Said by `#[rustx::main]`, with the table the build script generated.
#[doc(hidden)]
pub fn install_routes(routes: &'static [route::Route]) {
    ROUTES.with(|held| held.set(Some(routes)));
}

/// A window, and where its screens come from.
///
/// The defaults are what an application almost always wants:
/// `assets/index.html` for the document, `app` for the element the
/// screens are mounted in, `/` for the screen it opens on.
#[derive(Clone)]
pub struct Config {
    title: String,
    width: u32,
    height: u32,
    index: Option<PathBuf>,
    host: String,
    start: String,
    not_found: Option<fn() -> Element>,
    mounts: Vec<(String, fn() -> Element)>,
    hooks: Vec<std::rc::Rc<dyn Fn()>>,
}

impl Config {
    /// A window with this title, 1024 by 768.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 1024,
            height: 768,
            index: None,
            host: "app".to_string(),
            start: "/".to_string(),
            not_found: None,
            mounts: Vec::new(),
            hooks: Vec::new(),
        }
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// A document other than `assets/index.html`.
    pub fn index(mut self, path: impl Into<PathBuf>) -> Self {
        self.index = Some(path.into());
        self
    }

    /// The element the screens are mounted in. `app` by default.
    pub fn host(mut self, id: impl Into<String>) -> Self {
        self.host = id.into();
        self
    }

    /// The screen the window opens on. `/` by default.
    pub fn start(mut self, path: impl Into<String>) -> Self {
        self.start = path.into();
        self
    }

    /// The screen for a path nobody declared.
    pub fn not_found(mut self, component: fn() -> Element) -> Self {
        self.not_found = Some(component);
        self
    }

    /// Mount `component` in the element carrying this id.
    ///
    /// For an application that places its components itself rather than
    /// letting the router place its screens — a panel beside a header,
    /// each reading the same store. The mounting waits for the frame on
    /// which the element is there: a JavaScript context exists before
    /// the document's body does, and that is the framework's problem,
    /// not an application's.
    pub fn mount(mut self, id: impl Into<String>, component: fn() -> Element) -> Self {
        self.mounts.push((id.into(), component));
        self
    }

    /// Work to do when the page's JavaScript context appears — an
    /// application's own natives, the components it lets React mount.
    ///
    /// It runs in the render process, which is why it is given here and
    /// not called from `main`: by the time `main` has a window, that
    /// process has already come and gone.
    pub fn on_context_created(mut self, hook: impl Fn() + 'static) -> Self {
        self.hooks.push(std::rc::Rc::new(hook));
        self
    }
}

/// An application whose configuration has been checked.
pub struct App {
    inner: rustx_engine::App,
}

impl App {
    /// Check the configuration, and — in a sub-process — leave.
    ///
    /// CEF runs its render and utility processes by starting this same
    /// binary again, and such a process must do its work and exit
    /// without opening anything. That happens here, after the
    /// application's own hooks are registered and before anything else,
    /// because the render process is where those hooks run.
    pub fn new(config: Config) -> Result<Self, Error> {
        let routes = ROUTES.with(Cell::get);
        let host = config.host.clone();
        let start = config.start.clone();
        let not_found = config.not_found.unwrap_or(no_such_page);

        if let Some(routes) = routes {
            rustx_engine::on_context_created(move || start_router(routes, &host, &start, not_found));
        }
        for (id, component) in config.mounts {
            rustx_engine::on_context_created(move || mount_when_ready(id.clone(), component));
        }
        for hook in config.hooks {
            rustx_engine::on_context_created(move || hook());
        }

        if let rustx_engine::Bootstrap::Child { exit_code } = rustx_engine::bootstrap() {
            std::process::exit(exit_code);
        }

        let index = match config.index {
            Some(index) => index,
            None => manifest_dir().join("assets").join("index.html"),
        };

        Ok(Self {
            inner: rustx_engine::App::new(rustx_engine::AppConfig {
                title: config.title,
                width: config.width,
                height: config.height,
                index,
            })?,
        })
    }

    /// Open the window and run until it closes.
    pub fn run(self) -> Result<(), Error> {
        self.inner.run()
    }
}

/// The router is started on the first frame where its host element is
/// there: a JavaScript context exists before the document's body does,
/// and an application should not have to know that.
fn start_router(
    routes: &'static [route::Route],
    host: &str,
    start: &str,
    not_found: fn() -> Element,
) {
    let host = host.to_string();
    let start = start.to_string();
    let started = Cell::new(false);
    rustx_engine::on_frame(move || {
        if started.get() {
            return;
        }
        if router::start(routes, &host, &start, not_found).is_ok() {
            started.set(true);
        }
    });
}

/// Where a path nobody declared leads, unless the application says
/// otherwise.
fn no_such_page() -> Element {
    Element::tag("p")
        .attribute(
            "id",
            AttributeValue::Text("rustx-no-such-page".to_string()),
        )
        .child(Element::Text("This page does not exist.".to_string()))
        .build()
}

fn manifest_dir() -> PathBuf {
    MANIFEST_DIR
        .with(|held| held.borrow().clone())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Mount a component as soon as the element it goes in exists.
///
/// The same waiting the router does, for an application that places its
/// own components.
fn mount_when_ready(id: String, component: fn() -> Element) {
    let done = Cell::new(false);
    rustx_engine::on_frame(move || {
        if done.get() {
            return;
        }
        if render::mount_app(component, &id).is_ok() {
            done.set(true);
        }
    });
}
