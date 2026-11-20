//! The window: a tao event loop, a wry webview, and the rules of
//! [`crate::policy`] on every door into the host.
//!
//! Everything here runs on the thread that calls [`run`], which for tao on
//! Windows and macOS has to be the main thread of the process. Callbacks
//! are therefore not `Send`, and the state they close over can stay in
//! `Rc` and thread-locals as it does in the CEF engine.

use std::borrow::Cow;
use std::convert::Infallible;
use std::path::PathBuf;
use std::rc::Rc;

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::WindowBuilder;
use wry::http::{Response, header::CONTENT_TYPE};
use wry::{NewWindowResponse, WebContext, WebViewBuilder};

use crate::assets::serve;
use crate::policy::{Navigation, Policy};

#[derive(Debug, Clone)]
pub struct Config {
    pub title: String,
    pub size: (u32, u32),
    /// The folder the window's files are served from.
    pub root: PathBuf,
    /// The page loaded first, as a path: `/index.html`.
    pub entry: String,
    /// The custom protocol's name. It becomes the origin: see [`origin`].
    pub scheme: String,
    /// Origins that may load in the window. They get no native access.
    pub allow: Vec<String>,
    /// Where the webview keeps its profile. `None` lets wry choose, which
    /// on Windows is a folder next to the executable: fixed by
    /// [`default_data_dir`] for an installed application.
    pub data_dir: Option<PathBuf>,
    pub devtools: bool,
}

impl Config {
    pub fn new(title: &str, root: impl Into<PathBuf>) -> Self {
        Self {
            title: title.to_string(),
            size: (900, 640),
            root: root.into(),
            entry: "/index.html".to_string(),
            scheme: "rustx".to_string(),
            allow: Vec::new(),
            data_dir: default_data_dir(title),
            devtools: cfg!(debug_assertions),
        }
    }
}

/// Where a profile belongs on Windows: under the user's local app data, so
/// that an application installed in Program Files can write it. Other
/// platforms keep wry's own choice, which is already a user folder.
pub fn default_data_dir(app: &str) -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let base = std::env::var_os("LOCALAPPDATA")?;
    let safe: String = app
        .chars()
        .map(|c| match c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            true => c,
            false => '_',
        })
        .collect();
    Some(PathBuf::from(base).join(safe).join("webview"))
}

/// The origin pages of the custom protocol have, as the webview writes
/// it: Windows serves a custom protocol as `http://<name>.localhost`, the
/// other platforms as `<name>://localhost`.
pub fn origin(scheme: &str) -> String {
    match cfg!(windows) {
        true => format!("http://{scheme}.localhost"),
        false => format!("{scheme}://localhost"),
    }
}

#[derive(Debug)]
pub struct HostError(pub String);

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HostError {}

enum Command {
    Exit,
}

/// A way to stop the loop from anywhere: it may be cloned and sent.
#[derive(Clone)]
pub struct Control(EventLoopProxy<Command>);

impl Control {
    pub fn exit(&self) {
        let _ = self.0.send_event(Command::Exit);
    }
}

/// What the host calls back, on the loop thread.
pub struct Hooks {
    /// Called once, before the loop starts, with the way to stop it.
    pub on_start: Box<dyn Fn(&Control)>,
    /// A message from the application's own page, after the origin check.
    pub on_message: Box<dyn Fn(&str, &Control)>,
    /// An address to give to the system browser. The default is
    /// [`rustx_os::shell::open_url`]; a test puts its own.
    pub open_external: Box<dyn Fn(&str)>,
}

impl Default for Hooks {
    fn default() -> Self {
        Self {
            on_start: Box::new(|_| {}),
            on_message: Box::new(|_, _| {}),
            open_external: Box::new(|url| {
                if let Err(error) = rustx_os::shell::open_url(url) {
                    eprintln!("rustx: could not open the link: {error}");
                }
            }),
        }
    }
}

/// Open the window and run until it closes. Returns only on an error
/// before the loop starts: once it runs, closing the window ends the
/// process.
pub fn run(config: Config, hooks: Hooks) -> Result<Infallible, HostError> {
    let policy = Policy::new(
        &origin(&config.scheme),
        &config.allow.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let event_loop = EventLoopBuilder::<Command>::with_user_event().build();
    let control = Control(event_loop.create_proxy());
    let window = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(LogicalSize::new(config.size.0, config.size.1))
        .build(&event_loop)
        .map_err(|error| HostError(format!("could not open the window: {error}")))?;

    let hooks = Rc::new(hooks);
    let policy = Rc::new(policy);

    let root = config.root.clone();
    let mut context = WebContext::new(config.data_dir.clone());
    let builder = WebViewBuilder::new_with_web_context(&mut context)
        .with_devtools(config.devtools)
        .with_custom_protocol(config.scheme.clone(), move |_id, request| {
            let served = serve(&root, request.uri().path());
            Response::builder()
                .status(served.status)
                .header(CONTENT_TYPE, served.mime)
                .header("X-Content-Type-Options", "nosniff")
                .body(Cow::Owned(served.body))
                .unwrap_or_else(|_| Response::new(Cow::Borrowed(&b""[..])))
        })
        .with_ipc_handler({
            let (policy, hooks, control) = (Rc::clone(&policy), Rc::clone(&hooks), control.clone());
            move |request| {
                let from = request.uri().to_string();
                match policy.ipc_allowed(&from) {
                    true => (hooks.on_message)(request.body(), &control),
                    false => eprintln!(
                        "rustx: ignored a message from {}: only the application's own \
                         pages may call the host",
                        from.chars().take(40).collect::<String>()
                    ),
                }
            }
        })
        .with_navigation_handler({
            let (policy, hooks) = (Rc::clone(&policy), Rc::clone(&hooks));
            move |url| decide(&policy, &hooks, &url)
        })
        .with_new_window_req_handler({
            let (policy, hooks) = (Rc::clone(&policy), Rc::clone(&hooks));
            move |url, _features| {
                // A window of its own is never opened: a page asking for
                // one gets the system browser, or nothing.
                decide(&policy, &hooks, &url);
                NewWindowResponse::Deny
            }
        })
        .with_url(format!("{}{}", origin(&config.scheme), config.entry));

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let webview = builder.build(&window).map_err(|error| {
        // On Windows the usual cause is the missing runtime, and the
        // system's answer is an HRESULT that does not say so.
        let hint = match cfg!(windows) {
            true => {
                " The Microsoft Edge WebView2 Runtime may be missing or damaged: install \
                 it from developer.microsoft.com/microsoft-edge/webview2."
            }
            false => "",
        };
        HostError(format!("could not start the webview: {error}.{hint}"))
    })?;
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window
            .default_vbox()
            .ok_or_else(|| HostError("the window has no container for the webview".to_string()))?;
        builder
            .build_gtk(vbox)
            .map_err(|error| HostError(format!("could not start the webview: {error}")))?
    };

    (hooks.on_start)(&control);

    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        let _ = (&webview, &window, &context);
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            }
            | Event::UserEvent(Command::Exit) => *flow = ControlFlow::Exit,
            _ => {}
        }
    })
}

/// The policy's answer to an address, acted on. `true` means load it here.
fn decide(policy: &Policy, hooks: &Hooks, url: &str) -> bool {
    match policy.navigation(url) {
        Navigation::Stay => true,
        Navigation::OpenExternally => {
            (hooks.open_external)(url);
            false
        }
        Navigation::Refuse(why) => {
            eprintln!("rustx: refused to navigate: {why}");
            false
        }
    }
}
