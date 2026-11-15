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

// The two crates the compiler's generated code names —
// `rustx_dom::Element`, `rustx_engine::task::spawn`. They come through
// the facade so that an application declares one crate in its
// `Cargo.toml`, which is what the documentation promises. Nothing for
// the developer to write: `#[rustx::main]`'s prelude imports them.
#[doc(hidden)]
pub use rustx_dom;
#[doc(hidden)]
pub use rustx_engine;

/// The rest of the runtime, for an application that wants more than
/// time: `rustx::tokio::sync`, `rustx::tokio::select!`.
pub use tokio;
/// What an `await` waits on.
///
/// `rustx::time::sleep(…)`, and everything tokio calls time. A page
/// does not name tokio in its `Cargo.toml`: it is the same runtime its
/// `task(async { … })` blocks run on, and two tokios in one process
/// would be two reactors with the timers in only one of them.
pub use tokio::time;

pub use rustx_dom::value::{ToValue, record};
pub use rustx_dom::{
    AttributeValue, Element, Handler, IntoElement, Key, TagElement, Value, into_element, route,
    state, store, value,
};
pub use rustx_os::OsError;
/// What touches the machine rather than the page: the clipboard, file
/// dialogs, the devices, the system tray, notifications, the keychain,
/// the screens, the speaker. Each of them says, before it is asked for
/// anything, whether this platform can do it.
///
/// Every system module is here under its own short name except three,
/// which stay behind `rustx::os::` for a reason: `os::window`, because
/// `rustx::window` is what the page commands; `os::blocking` and
/// `os::patience`, which are for the code that calls these modules
/// rather than for a page.
pub use rustx_os::{
    about, attention, audio, autostart, bluetooth, capture, clipboard, color, devices, dialog,
    instance, menu, net, notify, print, screen, secret, shell, shortcut, system, terminal, tray,
};

/// The layer that talks to the system, in full.
///
/// The modules above exist so that a page can write
/// `rustx::clipboard::text()` with nothing imported. `rustx::os` is for
/// the rest: `os::window`, which holds the window's handle and cannot
/// be called `rustx::window` — that name belongs to what the **page**
/// commands, and two things cannot carry one name.
pub use rustx_os as os;

pub use rustx_engine::{
    Document, DomError, NativeError, Node, PostError, Poster, dropped, foreign, mount, mount_at_id,
    on_context_created, on_frame, poster, register_native, render, router, task, window,
};

/// Where this application keeps its own files, and what it writes
/// there.
///
/// The same functions as [`rustx::os::data`](rustx_os::data), with the
/// application's name already filled in -- which is the whole
/// difference: `rustx::data::write("settings.json", …)` is what a page
/// writes, and the folder it lands in is named after
/// `Config::new("…")`.
///
/// ```ignore
/// let seen = rustx::data::read_text("seen.json")?.unwrap_or_default();
/// rustx::data::write_text("seen.json", &fresh)?;
/// ```
///
/// This is what `localStorage` is for on the web and what
/// `app.getPath("userData")` is for in Electron. It is not the
/// engine's profile folder: that one belongs to Chromium, and an
/// application that writes in it is one Chromium may clear.
pub mod data {
    use std::path::PathBuf;

    use rustx_os::OsError;

    /// The name the folder is under: the application's own, as
    /// `Config::new` was given it.
    fn app() -> String {
        let named = rustx_engine::window::about().name;
        if named.is_empty() {
            // Before `App::new` has run -- a test, a tool -- there is
            // no title yet. A folder nobody chose is better than a
            // panic, and it is the same one every time.
            "rustx-app".to_string()
        } else {
            named
        }
    }

    /// What the user would mind losing.
    pub fn directory() -> Result<PathBuf, OsError> {
        rustx_os::data::directory(&app())
    }

    /// What can be deleted at any moment.
    pub fn cache() -> Result<PathBuf, OsError> {
        rustx_os::data::cache(&app())
    }

    /// Where to write what somebody will be asked to send.
    pub fn logs() -> Result<PathBuf, OsError> {
        rustx_os::data::logs(&app())
    }

    /// The path of one file in [`directory`], without touching it.
    pub fn path(name: &str) -> Result<PathBuf, OsError> {
        rustx_os::data::path(&app(), name)
    }

    /// The bytes of a file, or `None` when it has never been written.
    pub fn read(name: &str) -> Result<Option<Vec<u8>>, OsError> {
        rustx_os::data::read(&app(), name)
    }

    /// The same, as text.
    pub fn read_text(name: &str) -> Result<Option<String>, OsError> {
        rustx_os::data::read_text(&app(), name)
    }

    /// Write a file, whole or not at all.
    pub fn write(name: &str, bytes: &[u8]) -> Result<(), OsError> {
        rustx_os::data::write(&app(), name, bytes)
    }

    /// The same, from text.
    pub fn write_text(name: &str, text: &str) -> Result<(), OsError> {
        rustx_os::data::write_text(&app(), name, text)
    }

    /// Forget a file. One that was never written is not an error.
    pub fn remove(name: &str) -> Result<(), OsError> {
        rustx_os::data::remove(&app(), name)
    }

    /// What this application has saved, by name.
    pub fn list() -> Result<Vec<String>, OsError> {
        rustx_os::data::list(&app())
    }
}

/// Updating the application from its own release page.
///
/// [`rustx::os::update`](rustx_os::update) with the application's name
/// filled in, and `current` read from the crate that is running rather
/// than passed in -- which is the one thing the facade can do and the
/// layer below it cannot.
///
/// ```ignore
/// // In a handler, or a task: it reaches the network.
/// match rustx::update::from_github("aztra/mon-app")? {
///     rustx::update::Outcome::UpToDate => note = "up to date".to_string(),
///     rustx::update::Outcome::Installed(release) => {
///         note = format!("{} is installed. Restart to use it.", release.version);
///     }
/// }
/// ```
///
/// Nothing here happens on its own, and nothing restarts the
/// application for you: when it is safe to restart is a question about
/// unsaved work, which only the application can answer. [`restart`]
/// does it when the answer is yes.
pub mod update {
    use rustx_os::OsError;
    pub use rustx_os::update::{Outcome, Release, clean, restart};

    fn app() -> String {
        let named = rustx_engine::window::about().name;
        if named.is_empty() {
            "rustx-app".to_string()
        } else {
            named
        }
    }

    /// The version this build says it is -- `Config::version(…)`, which
    /// `rustx build` also stamps into the binary.
    fn running() -> String {
        rustx_engine::window::about().version
    }

    /// Is there a newer release than the one running?
    ///
    /// `repository` is `owner/name`, as the address bar writes it. The
    /// answer is `None` both when the newest release is this one and
    /// when it has no archive for this machine.
    pub fn newest(repository: &str) -> Result<Option<Release>, OsError> {
        let Some(release) = rustx_os::update::latest(repository, &app())? else {
            return Ok(None);
        };
        Ok(release.is_newer_than(&running()).then_some(release))
    }

    /// Check, download and install -- and do nothing at all when there
    /// is nothing newer.
    ///
    /// It blocks: it talks to a release page and may move three hundred
    /// megabytes. In a page that means inside a `task(…)` or an
    /// `async` block, like every other piece of work that waits.
    pub fn from_github(repository: &str) -> Result<Outcome, OsError> {
        rustx_os::update::from_github(repository, &app(), &running(), &mut |_, _| {})
    }

    /// The same, with the bytes so far and the bytes expected handed to
    /// `progress` as the download lands -- which is what a page draws a
    /// bar from. It is called on the thread that called this, so the
    /// number crosses to the screen the way everything else does: by
    /// being posted.
    pub fn from_github_watching(
        repository: &str,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<Outcome, OsError> {
        rustx_os::update::from_github(repository, &app(), &running(), progress)
    }
}

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
    /// Everything under `assets/`, as the attribute read it at compile
    /// time, so that an application carries its own files rather than
    /// reading them from the folder it was built in.
    static ASSETS: Cell<&'static [Asset]> = const { Cell::new(&[]) };
    /// Where those files were put on this machine, worked out once.
    static ASSETS_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// One file of `assets/`, carried inside the executable.
///
/// The path is relative to `assets/` and always written with forward
/// slashes, because it is both a file name and a piece of a URL.
pub type Asset = (&'static str, &'static [u8]);

/// Wait for a future to the end, from `main`.
///
/// What `#[rustx::main]` wraps an `async fn main` in. A component never
/// waits: it is asked for the screen as it is now, and what waits is
/// written in a `task(async { … })`.
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

/// Said by `#[rustx::main]`: every file under `assets/`, with its
/// contents, as they were when the application was compiled.
///
/// This is what makes a built application work anywhere. The document,
/// the JavaScript bundle, the window's icon and every image a page
/// names used to be read from the directory cargo happened to compile
/// in -- so the application ran on the machine that built it and showed
/// a blank window everywhere else. They travel inside the executable
/// now.
#[doc(hidden)]
pub fn install_assets(assets: &'static [Asset]) {
    ASSETS.with(|held| held.set(assets));
}

/// Where this application's `assets/` are, on this machine.
///
/// Three places, in this order, and the reason for the order is in
/// each:
///
/// 1. **Beside the executable.** A packaged application has its files
///    there, and so does one whose `assets/` somebody has edited after
///    shipping it -- a logo swapped for a customer, a document patched
///    on a machine that cannot rebuild. What is on disk beside the
///    program wins over what is inside it.
/// 2. **Unpacked from the executable**, into
///    `%LOCALAPPDATA%\rustx\<title>\assets` (or the system's
///    temporary folder when there is no such place). Written once and
///    then only when the bytes differ, so a start that changes nothing
///    writes nothing.
/// 3. **The directory it was compiled in**, which is what `rustx dev`
///    is: the files are right there, and reading them means an edit
///    shows up without a rebuild.
pub fn assets_dir() -> PathBuf {
    if let Some(known) = ASSETS_DIR.with(|held| held.borrow().clone()) {
        return known;
    }
    let found = found_assets_dir();
    ASSETS_DIR.with(|held| *held.borrow_mut() = Some(found.clone()));
    found
}

fn found_assets_dir() -> PathBuf {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|directory| directory.join("assets")));
    if let Some(beside) = beside
        && beside.is_dir()
    {
        return beside;
    }

    let carried = ASSETS.with(|held| held.get());
    if !carried.is_empty()
        && let Some(unpacked) = unpacked_assets(carried)
    {
        return unpacked;
    }

    manifest_dir().join("assets")
}

/// Write the carried files out, and say where they are.
///
/// Only what differs: a file whose bytes are already on disk is left
/// alone, so the folder's timestamps mean something and a start costs
/// nothing. A file that cannot be written is skipped rather than fatal
/// -- an application whose icon could not be unpacked should still open
/// its window.
fn unpacked_assets(carried: &'static [Asset]) -> Option<PathBuf> {
    let directory = unpacking_directory()?;
    for (name, bytes) in carried {
        let path = directory.join(name);
        if let Ok(already) = std::fs::read(&path)
            && already == *bytes
        {
            continue;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, bytes);
    }
    Some(directory)
}

fn unpacking_directory() -> Option<PathBuf> {
    let slug = rustx_engine::window::about().name;
    let slug = slugged(if slug.is_empty() { "rustx-app" } else { &slug });
    let root = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let directory = root.join("rustx").join(slug).join("assets");
    std::fs::create_dir_all(&directory).ok()?;
    Some(directory)
}

/// A title as a folder name: lowercase, and one dash where anything
/// that is not a letter or a digit was.
fn slugged(title: &str) -> String {
    let mut out = String::new();
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
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
    failed: Option<fn(String) -> Element>,
    mounts: Vec<(String, fn() -> Element)>,
    hooks: Vec<std::rc::Rc<dyn Fn()>>,
    icon: Option<PathBuf>,
    about: rustx_engine::window::About,
    dressing: rustx_os::window::Dressing,
    scrollbars: bool,
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
            failed: None,
            mounts: Vec::new(),
            hooks: Vec::new(),
            icon: None,
            about: rustx_engine::window::About {
                name: title,
                ..Default::default()
            },
            dressing: rustx_os::window::Dressing::System,
            scrollbars: true,
        }
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// A window without the system's frame: no title bar, no buttons,
    /// the page draws all of it.
    ///
    /// What then has to be given back to the user, which the frame used
    /// to do: `window::drag()` on a press of the bar the page drew,
    /// `window::minimize()`, `window::maximize()` and `window::shut()`
    /// on its three buttons. Without `drag`, the window no longer
    /// moves.
    pub fn frameless(mut self) -> Self {
        self.dressing = rustx_os::window::Dressing::None;
        self
    }

    /// A window that scrolls without drawing a scrollbar.
    ///
    /// Nothing stops scrolling: the wheel, the trackpad, the arrow
    /// keys, Page Down, a drag against the edge and
    /// `scrollIntoView` all work exactly as before. The bar itself is
    /// not painted, and it takes no width, so a panel that was 8
    /// pixels narrower than its neighbour because one of them
    /// overflowed lines up again.
    ///
    /// This is the look a desktop application usually wants and the
    /// one a browser does not: a document on the web says how far down
    /// it goes with a scrollbar, and an application says it with its
    /// own design — a fading edge, a thumb drawn in the page, a list
    /// that is obviously a list. The web's default is the right
    /// default, which is why this is asked for rather than assumed.
    ///
    /// The whole window, every element in it, and inside a React
    /// subtree too — one stylesheet, put in after the application's
    /// own and marked `!important`. Calling this is a more deliberate
    /// statement than any rule a component library happened to ship,
    /// so it wins over them: a `::-webkit-scrollbar { width: 8px }`
    /// three levels deep in somebody's CSS does not quietly put the
    /// bar back. An application that wants bars in one panel and not
    /// elsewhere does not call this and styles that panel itself.
    ///
    /// ```ignore
    /// rustx::Config::new("Console").frameless().without_scrollbars()
    /// ```
    pub fn without_scrollbars(mut self) -> Self {
        self.scrollbars = false;
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
    ///
    /// There is one without this call — the engine's own, which says so
    /// and says where routes come from. This is for an application that
    /// wants the screen to look like the rest of it.
    pub fn not_found(mut self, component: fn() -> Element) -> Self {
        self.not_found = Some(component);
        self
    }

    /// The screen for a page that fell over, given what it said.
    ///
    /// The last of three answers, and the first two win over it: an
    /// `error.rsx` beside a page is that page's frontier, this is the
    /// application's, and the engine's own is what an application that
    /// wrote neither is given. None of the three is a blank window,
    /// which is what a page that panicked used to leave behind.
    pub fn on_error(mut self, component: fn(String) -> Element) -> Self {
        self.failed = Some(component);
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
        // The engine brings both fallback screens; an application that
        // named one replaces that one and keeps the other.
        let screens = router::Screens {
            not_found: config
                .not_found
                .unwrap_or(router::Screens::default().not_found),
            failed: config.failed.unwrap_or(router::Screens::default().failed),
        };

        // Before anything renders: what the process this one replaced was
        // holding, when `rustx dev` is the one that started it. Nothing
        // else sets `RUSTX_CARRIED`, so outside a watching loop this is
        // one lookup and nothing more.
        rustx_engine::on_context_created(carry_state);

        if let Some(routes) = routes {
            rustx_engine::on_context_created(move || start_router(routes, &host, &start, screens));
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
        let scrollbars = config.scrollbars;
        rustx_engine::on_context_created(move || {
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
            // And the scrollbars, hidden if the application asked for
            // that — last, so that a page which styles
            // `::-webkit-scrollbar` on purpose is not overruled by a
            // sheet it did not write.
            if !scrollbars {
                rustx_engine::inject("app-scrollbars", &hidden_scrollbars());
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
                .map(|icon| resolved_asset(&icon))
                .or_else(default_icon),
            config.about,
        );

        if let rustx_engine::Bootstrap::Child { exit_code } = rustx_engine::bootstrap() {
            std::process::exit(exit_code);
        }

        let index = match config.index {
            Some(index) => resolved_asset(&index),
            None => document_for(&assets_dir(), &config.title)?,
        };

        Ok(Self {
            inner: rustx_engine::App::new(rustx_engine::AppConfig {
                title: config.title,
                width: config.width,
                height: config.height,
                dressing: config.dressing,
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
    screens: router::Screens,
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
        if router::start_with(routes, &host, &start, screens).is_ok() {
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
    std::fs::write(&path, default_document(title, &manifest_dir.join("assets")))
        .map_err(|_| Error::IndexNotFound(path.clone()))?;
    Ok(path)
}

/// A directory as a URL a document can resolve relative paths against.
///
/// `C:\Users\me\app\assets` becomes `file:///C:/Users/me/app/assets/`.
/// The trailing slash is what makes it a directory rather than a file
/// to replace: without it, `rustx-bundle.js` would be looked for beside
/// `assets`, not inside it.
fn directory_url(directory: &Path) -> String {
    let mut url = String::from("file:///");
    for byte in directory.to_string_lossy().as_bytes() {
        match byte {
            b'\\' => url.push('/'),
            // The characters a path is allowed to hold and a URL reads
            // as something else. A space is the one that actually
            // happens — `C:\Users\Jean Pierre\…` — and a document that
            // stopped at the space looked for a bundle in a folder
            // nobody has.
            b'%' => url.push_str("%25"),
            b' ' => url.push_str("%20"),
            b'#' => url.push_str("%23"),
            b'?' => url.push_str("%3F"),
            b'\'' => url.push_str("%27"),
            b'"' => url.push_str("%22"),
            // Anything outside plain ASCII goes in as the UTF-8 bytes
            // it is, percent by percent: a path through `Développeur`
            // is a path, and pushing its bytes as `char`s would have
            // written mojibake into the URL.
            0x00..=0x20 | 0x7f..=0xff => url.push_str(&format!("%{byte:02X}")),
            _ => url.push(*byte as char),
        }
    }
    // An absolute Unix path already began with the separator, so the
    // `file:///` above would have given it four.
    while url.starts_with("file:////") {
        url.replace_range(7..8, "");
    }
    if !url.ends_with('/') {
        url.push('/');
    }
    url
}

/// Enough to tell two applications apart without naming their paths in
/// a filename.
fn hash_of(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

/// The stylesheet that hides every scrollbar and stops none of the
/// scrolling.
///
/// Two rules, because they are two different mechanisms and Chromium
/// honours both: `scrollbar-width: none` is the standard property, and
/// the `::-webkit-scrollbar` pseudo-element is what a page that styles
/// its own bars is written against — a sheet that sets a width there
/// would otherwise put the bar back, which is why both are `!important`
/// and why this goes in last.
///
/// `html` and `body` are named as well as `*`, because the viewport's
/// own scrollbar belongs to the document element and not to any
/// element inside it.
///
/// Put in with the same waiting as the stylesheet: a JavaScript
/// context exists before the document's `<head>` does, and a window
/// that is never composited never calls `requestAnimationFrame` back,
/// so this waits on a timer rather than on a frame.
fn hidden_scrollbars() -> String {
    const CSS: &str = "\
        html, body, * { scrollbar-width: none !important; \
        -ms-overflow-style: none !important; } \
        html::-webkit-scrollbar, body::-webkit-scrollbar, *::-webkit-scrollbar { \
        width: 0 !important; height: 0 !important; display: none !important; }";
    format!(
        "(() => {{ const css = {}; const put = () => {{ \
         if (!document.head) {{ setTimeout(put, 16); return; }} \
         const sheet = document.createElement('style'); \
         sheet.id = 'rustx-no-scrollbars'; sheet.textContent = css; \
         document.head.appendChild(sheet); }}; put(); }})();",
        javascript_string(CSS)
    )
}

/// The document an application that wrote none is given.
///
/// With a `<base>` pointing at the application's own `assets`, because
/// this document is not written there — it goes in a directory of its
/// own under the system's temporary files, so that two applications
/// running at once are never handed the same file. Nothing else is in
/// that directory, so every relative URL in the page resolved to
/// nothing: the one the framework itself writes, `rustx-bundle.js`,
/// loaded in an application that had an `assets/index.html` and silently
/// did not in an application that had none. A page whose libraries never
/// arrived says `the page asked for \`LineChart\`, and nothing has it`,
/// which is true and tells the developer to go and look in the wrong
/// place. The `<base>` is what makes the generated document resolve
/// paths the way a document written beside the assets would.
fn default_document(title: &str, assets: &Path) -> String {
    let title = title.replace('<', "&lt;").replace('&', "&amp;");
    let base = directory_url(assets);
    format!(
        "<!doctype html>\n<html>\n  <head>\n    <meta charset=\"utf-8\" />\n    \
         <base href=\"{base}\" />\n    \
         <title>{title}</title>\n    <style>\n      \
         :root {{ color-scheme: light dark; }}\n      \
         body {{ font-family: system-ui, sans-serif; margin: 0; }}\n    \
         </style>\n  </head>\n  <body>\n    \
         <!-- The screens go here. They are written in app/. -->\n    \
         <div id=\"app\"></div>\n  </body>\n</html>\n"
    )
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
    let obvious = assets_dir().join("icon.ico");
    obvious.exists().then_some(obvious)
}

/// A path an application wrote in its `Config`, resolved where the
/// files actually are.
///
/// `Config::icon("assets/icon.ico")` and
/// `Config::index("assets/index.html")` are written relative to the
/// project, because that is where the developer is looking when they
/// write them. At run time `assets/` may be beside the executable or
/// unpacked out of it, so the `assets/` in front is replaced by
/// wherever `assets/` is. An absolute path is the application being
/// explicit, and is taken as it is.
fn resolved_asset(written: &Path) -> PathBuf {
    if written.is_absolute() {
        return written.to_path_buf();
    }
    let relative = written
        .strip_prefix("assets")
        .or_else(|_| written.strip_prefix("./assets"))
        .unwrap_or(written);
    let inside = assets_dir().join(relative);
    if inside.exists() {
        return inside;
    }
    manifest_dir().join(written)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_draws_its_scrollbars_unless_it_is_asked_not_to() {
        // The web's default is the right default: a document says how
        // far down it goes with a scrollbar, and only an application
        // that has drawn its own way of saying so should take it away.
        assert!(Config::new("Console").scrollbars);
        assert!(!Config::new("Console").without_scrollbars().scrollbars);
    }

    #[test]
    fn hiding_the_scrollbars_does_not_stop_the_scrolling() {
        let css = hidden_scrollbars();
        // Width and visibility, and nothing that touches `overflow`:
        // a rule that set `overflow: hidden` would hide the bar by
        // taking the scrolling away with it, which is the one thing
        // this must not do.
        assert!(css.contains("scrollbar-width"), "got {css}");
        assert!(css.contains("::-webkit-scrollbar"), "got {css}");
        assert!(
            !css.contains("overflow:"),
            "this must not stop scrolling: {css}"
        );
    }

    #[test]
    fn the_viewports_own_scrollbar_is_named_and_not_only_the_wildcard() {
        // The window's bar belongs to the document element, not to any
        // element inside it, so `*` alone would leave it drawn.
        let css = hidden_scrollbars();
        assert!(css.contains("html"), "got {css}");
        assert!(css.contains("body"), "got {css}");
    }

    #[test]
    fn the_rule_outranks_a_stylesheet_that_draws_its_own_bars() {
        // Calling `without_scrollbars` is more deliberate than a rule
        // a component library happened to ship.
        let css = hidden_scrollbars();
        assert!(css.contains("!important"), "got {css}");
    }

    #[test]
    fn the_stylesheet_waits_for_a_head_to_put_itself_in() {
        // A JavaScript context exists before the document's `<head>`
        // does, and a window nothing composites never calls
        // `requestAnimationFrame` back — so this waits on a timer.
        let css = hidden_scrollbars();
        assert!(css.contains("document.head"), "got {css}");
        assert!(css.contains("setTimeout"), "got {css}");
        assert!(!css.contains("requestAnimationFrame"), "got {css}");
    }
}
