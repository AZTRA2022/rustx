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
use std::path::{Path, PathBuf};

pub use rustx_macros::{ToValue, main};

// Les deux caisses que le code engendré par le compilateur nomme —
// `rustx_dom::Element`, `rustx_engine::task::spawn`. Elles passent par
// la façade pour qu'une application n'en déclare qu'une seule dans son
// `Cargo.toml`, ce que la documentation promet. Rien à écrire pour le
// développeur : le prélude de `#[rustx::main]` les importe.
#[doc(hidden)]
pub use rustx_dom;
#[doc(hidden)]
pub use rustx_engine;

/// Ce qu'un `await` attend.
///
/// `rustx::time::sleep(…)`, et tout ce que tokio appelle le temps. Une
/// page ne nomme pas tokio dans son `Cargo.toml` : c'est le meme
/// runtime que celui qui fait tourner ses `task(async { … })`, et deux
/// tokio dans un processus seraient deux reacteurs dont un seul aurait
/// les timers.
pub use tokio::time;
/// Le reste du runtime, pour une application qui en veut plus que le
/// temps : `rustx::tokio::sync`, `rustx::tokio::select!`.
pub use tokio;

pub use rustx_dom::value::{ToValue, record};
pub use rustx_dom::{
    AttributeValue, Element, Handler, IntoElement, Key, TagElement, Value, into_element, route,
    state, store, value,
};
/// Ce qui touche la machine et non la page : le presse-papiers, les
/// dialogues de fichiers, les appareils, la barre système, les
/// notifications. Chacun dit, avant qu'on lui demande quoi que ce soit,
/// si cette plateforme sait le faire.
pub use rustx_os::{autostart, clipboard, devices, dialog, instance, notify, shortcut, tray};
pub use rustx_os::OsError;

pub use rustx_engine::{
    Document, DomError, NativeError, Node, PostError, Poster, foreign, mount, mount_at_id,
    dropped, on_context_created, on_frame, poster, register_native, render, router, task, window,
};

/// What can go wrong while an application starts.
pub type Error = rustx_engine::EngineError;

/// A component of the application, by the name it was written with.
pub type NamedComponent = (&'static str, fn() -> Element);

/// A value the pages share, by the name it was written with, read as
/// text — which is all a scenario asks for.
pub type NamedValue = (&'static str, fn() -> String);

thread_local! {
    /// The directory of the crate being run, as `env!` saw it there.
    static MANIFEST_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    /// The route table the build script wrote.
    static ROUTES: Cell<Option<&'static [route::Route]>> = const { Cell::new(None) };
    /// Every component the pages declare, for React to mount by name.
    static COMPONENTS: RefCell<Vec<NamedComponent>> =
        const { RefCell::new(Vec::new()) };
    /// Every value the pages share, by name, for a scenario to read.
    static SHARED: RefCell<Vec<NamedValue>> = const { RefCell::new(Vec::new()) };
    /// The application.s own JavaScript, bundled by the attribute.
    static BUNDLE: Cell<Option<&'static str>> = const { Cell::new(None) };
    /// The stylesheet Tailwind wrote from the classes the pages use.
    static STYLESHEET: Cell<Option<&'static str>> = const { Cell::new(None) };
}

/// Attendre une future jusqu'au bout, depuis `main`.
///
/// Ce que `#[rustx::main]` enveloppe autour d'un `async fn main`. Un
/// composant, lui, n'attend jamais : on lui demande l'ecran tel qu'il
/// est maintenant, et ce qui attend s'ecrit dans un `task(async { … })`.
#[doc(hidden)]
pub fn block_on<T>(work: impl std::future::Future<Output = T>) -> T {
    rustx_engine::runtime::block_on(work)
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

/// Said by `#[rustx::main]`, with every component the pages declare.
///
/// An application does not choose which ones React may mount: a
/// component written in `app/` is a component, and a name nobody wrote
/// is an error React can catch.
#[doc(hidden)]
pub fn install_components(components: Vec<NamedComponent>) {
    COMPONENTS.with(|held| *held.borrow_mut() = components);
}

/// Said by `#[rustx::main]`, with every value the pages share.
///
/// It is what `rustx_shared("name")` answers from: a scenario reads a
/// value the page does not show without the application writing a
/// native of its own to look at its own state.
#[doc(hidden)]
pub fn install_shared_values(values: Vec<NamedValue>) {
    SHARED.with(|held| *held.borrow_mut() = values);
}

/// Said by `#[rustx::main]`, with the name of the application's own
/// JavaScript.
///
/// The attribute generated the entry point from `package.json`, bundled
/// it and put it beside the document. What goes in the page is the one
/// `<script src>` the developer does not have to write — a bundle is a
/// megabyte of JavaScript, and a browser loads that from a URL.
#[doc(hidden)]
pub fn install_bundle(name: &'static str) {
    BUNDLE.with(|held| held.set(Some(name)));
}

/// Said by `#[rustx::main]`, with the stylesheet Tailwind wrote from the
/// classes the pages use.
///
/// Compiled into the binary and added to the document at startup, so a
/// page that writes `className="bg-violet-500"` is styled without the
/// application shipping a `.css` beside its executable.
#[doc(hidden)]
pub fn install_stylesheet(css: &'static str) {
    STYLESHEET.with(|held| held.set(Some(css)));
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
    icon: Option<PathBuf>,
    about: rustx_engine::window::About,
}

impl Config {
    /// A window with this title, 1024 by 768.
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        Self {
            title: title.clone(),
            width: 1024,
            height: 768,
            index: None,
            host: "app".to_string(),
            start: "/".to_string(),
            not_found: None,
            mounts: Vec::new(),
            hooks: Vec::new(),
            icon: None,
            about: rustx_engine::window::About {
                name: title,
                ..Default::default()
            },
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

    /// The window's logo: an `.ico`, relative to the application's own
    /// directory.
    ///
    /// It goes in the title bar and in the taskbar. The *binary's* own
    /// icon — what a file manager shows before the application has even
    /// run — is a resource linked into the executable, which is the
    /// command line's work and not a library's.
    pub fn icon(mut self, path: impl Into<PathBuf>) -> Self {
        self.icon = Some(path.into());
        self
    }

    /// What the application says it is, for a window that shows it.
    ///
    /// A page reads it with `rustx_about()` rather than holding a copy
    /// of its own version: a number written twice is a number that is
    /// wrong in one of the two places.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.about.version = version.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.about.description = description.into();
        self
    }

    pub fn publisher(mut self, publisher: impl Into<String>) -> Self {
        self.about.publisher = publisher.into();
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
        let components = COMPONENTS.with(|held| held.borrow().clone());
        let host = config.host.clone();
        let start = config.start.clone();
        let not_found = config.not_found.unwrap_or(no_such_page);

        // Before anything renders: what the process this one replaced was
        // holding, when `rustx dev` is the one that started it. Nothing
        // else sets `RUSTX_CARRIED`, so outside a watching loop this is
        // one lookup and nothing more.
        rustx_engine::on_context_created(carry_state);

        if let Some(routes) = routes {
            rustx_engine::on_context_created(move || {
                start_router(routes, &host, &start, not_found)
            });
        }
        rustx_engine::on_context_created(move || {
            for (name, component) in &components {
                foreign::register_component(name, *component);
            }
        });
        rustx_engine::on_context_created(install_shared_native);
        rustx_engine::on_context_created(install_about_native);
        // The other half of the React boundary. An application says
        // which libraries it uses — `rustx.use({ recharts })` — and
        // writes its components in `.rsx`; the bridge between the two
        // is the framework's, so the framework brings it.
        rustx_engine::on_context_created(|| {
            rustx_engine::inject("react-runtime", include_str!("../assets/runtime.js"));
            // And the application's own libraries, after the runtime
            // that reads them. Both go in through the frame, in order.
            if let Some(name) = BUNDLE.with(Cell::get) {
                rustx_engine::inject(
                    "app-bundle",
                    &format!(
                        "(() => {{ const put = () => {{ \r
                         if (!document.head) {{ setTimeout(put, 16); return; }} \r
                         const script = document.createElement('script'); \r
                         script.id = 'rustx-bundle'; script.src = {}; \r
                         document.head.appendChild(script); }}; put(); }})();",
                        javascript_string(name)
                    ),
                );
            }
            // And the stylesheet, as a `<style>` the document did not
            // have to name either. JavaScript puts it there because
            // that is what the engine can put in a page.
            if let Some(css) = STYLESHEET.with(Cell::get) {
                rustx_engine::inject(
                    "app-stylesheet",
                    &format!(
                        // Waited for, not assumed: what the engine puts
                        // in a page runs when the JavaScript context is
                        // created, and a context exists before the
                        // document's `<head>` does. Waited for on a
                        // timer, not on a frame: a window whose
                        // compositor produces no frames never calls
                        // `requestAnimationFrame` back, and the page
                        // would have no stylesheet for ever.
                        "(() => {{ const css = {}; const put = () => {{ \
                         if (!document.head) {{ setTimeout(put, 16); return; }} \
                         const sheet = document.createElement('style'); \
                         sheet.id = 'rustx-stylesheet'; sheet.textContent = css; \
                         document.head.appendChild(sheet); }}; put(); }})();",
                        javascript_string(css)
                    ),
                );
            }
        });
        for (id, component) in config.mounts {
            rustx_engine::on_context_created(move || mount_when_ready(id.clone(), component));
        }
        for hook in config.hooks {
            rustx_engine::on_context_created(move || hook());
        }

        // The window's own logo and what the application says it is.
        // Declared before the sub-processes leave, because the render
        // process answers `rustx_about()` and it never comes back here.
        rustx_engine::window::declare(
            config
                .icon
                .map(|icon| manifest_dir().join(icon))
                .or_else(default_icon),
            config.about,
        );

        if let rustx_engine::Bootstrap::Child { exit_code } = rustx_engine::bootstrap() {
            std::process::exit(exit_code);
        }

        let index = match config.index {
            Some(index) => index,
            None => document_for(&manifest_dir(), &config.title)?,
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
    // The screen that was open when `rustx dev` replaced the last
    // process, if there was one: a developer who saves a file while
    // looking at the third screen of a form wants the third screen of
    // the form back, not the first.
    // Dans l'ordre : la route que cette fenêtre-ci porte dans son URL
    // — c'est ainsi qu'une deuxième fenêtre apprend la sienne, et elle
    // passe devant tout le reste parce que c'est une demande explicite
    // —, puis l'écran qui était ouvert quand `rustx dev` a remplacé le
    // dernier processus, puis la route de départ de l'application.
    let start = rustx_engine::window::page_asked_for()
        .or_else(rustx_dom::carried::page)
        .unwrap_or_else(|| start.to_string());
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

/// Read what `rustx dev` carried over, when it is `rustx dev` that
/// started this process.
///
/// The file holds the values that have a name and the page that was
/// open. Nothing else in the world sets `RUSTX_CARRIED`, so a shipped
/// application never reads or writes it.
fn carry_state() {
    if let Some(file) = std::env::var_os("RUSTX_CARRIED") {
        rustx_dom::carried::carrying(std::path::Path::new(&file));
    }
}

/// A string as JavaScript would write it.
///
/// A stylesheet is full of braces, quotes and newlines, and it is going
/// into a script: everything that could end the literal early has to be
/// escaped, or a `}` in a CSS rule ends the statement instead.
fn javascript_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for letter in text.chars() {
        match letter {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            '<' => out.push_str("\\u003c"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// The document the screens are put in.
///
/// `assets/index.html` when the application wrote one. When it did not,
/// the obvious one: a host element, the window's title, and nothing
/// else. A new application has pages to write, and the HTML that holds
/// them is the same HTML every time — so it is not the application's to
/// write.
///
/// The written document is named after the application and the process
/// that is running it, because two applications running at once must
/// not be handed the same file.
#[doc(hidden)]
pub fn document_for(manifest_dir: &Path, title: &str) -> Result<PathBuf, Error> {
    let written = manifest_dir.join("assets").join("index.html");
    if written.exists() {
        return Ok(written);
    }

    let slug: String = title
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let directory = std::env::temp_dir().join(format!(
        "rustx-document-{slug}-{}-{}",
        std::process::id(),
        hash_of(manifest_dir)
    ));
    let path = directory.join("index.html");
    std::fs::create_dir_all(&directory).map_err(|_| Error::IndexNotFound(path.clone()))?;
    std::fs::write(&path, default_document(title))
        .map_err(|_| Error::IndexNotFound(path.clone()))?;
    Ok(path)
}

/// Enough to tell two applications apart without naming their paths in
/// a filename.
fn hash_of(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

/// The document an application that wrote none is given.
fn default_document(title: &str) -> String {
    let title = title.replace('<', "&lt;").replace('&', "&amp;");
    format!(
        "<!doctype html>\n<html>\n  <head>\n    <meta charset=\"utf-8\" />\n    \
         <title>{title}</title>\n    <style>\n      \
         :root {{ color-scheme: light dark; }}\n      \
         body {{ font-family: system-ui, sans-serif; margin: 0; }}\n    \
         </style>\n  </head>\n  <body>\n    \
         <!-- The screens go here. They are written in app/. -->\n    \
         <div id=\"app\"></div>\n  </body>\n</html>\n"
    )
}

/// Where a path nobody declared leads, unless the application says
/// otherwise.
fn no_such_page() -> Element {
    Element::tag("p")
        .attribute("id", AttributeValue::Text("rustx-no-such-page".to_string()))
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

/// `rustx_shared("points")` — the value the page declared under that
/// name, as text.
///
/// A scenario needs to look at state the page does not show: a count
/// after the panel that displayed it has gone, a flag a handler set.
/// Every application was writing a native of its own for that. The
/// values have names, so the framework can answer.
fn install_shared_native() {
    register_native("rustx_shared", |arguments| {
        let [Value::String(name)] = arguments else {
            return Err(NativeError::new(
                "rustx_shared takes the name a value was declared with",
            ));
        };
        let found = SHARED.with(|held| {
            held.borrow()
                .iter()
                .find(|(declared, _)| declared == name)
                .map(|(_, read)| *read)
        });
        match found {
            Some(read) => Ok(Value::String(read())),
            None => Err(NativeError::new(format!(
                "no page declares a value called `{name}` that could be shown as text"
            ))),
        }
    });
}

/// `assets/icon.ico`, when the application has one and did not say so.
///
/// The same courtesy as `assets/index.html`: a file in the obvious place
/// is a file the framework finds, and an application that wants another
/// one says which with `Config::icon`.
fn default_icon() -> Option<PathBuf> {
    let obvious = manifest_dir().join("assets").join("icon.ico");
    obvious.exists().then_some(obvious)
}

/// `rustx_about()`: what the application says it is.
///
/// A page that shows its own version reads it from here rather than
/// holding a copy: a number written twice is a number that is wrong in
/// one of the two places.
fn install_about_native() {
    rustx_engine::register_native("rustx_about", |_arguments| {
        let about = rustx_engine::window::about();
        Ok(Value::Record(vec![
            ("name".to_string(), Value::String(about.name)),
            ("version".to_string(), Value::String(about.version)),
            ("description".to_string(), Value::String(about.description)),
            ("publisher".to_string(), Value::String(about.publisher)),
        ]))
    });
}
