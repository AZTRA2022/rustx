//! The CEF application object.
//!
//! One object, two jobs, because CEF hands the same `App` to both halves of
//! the relaunch. In the browser process it opens the window; in the render
//! process it installs the application's Rust functions on the JavaScript
//! global. CEF will not let a browser be created straight after `initialize`,
//! so the window is opened from `on_context_initialized`.

use cef::{
    App, Browser, BrowserProcessHandler, BrowserSettings, CefString, CommandLine, Frame, ImplApp,
    ImplBrowserProcessHandler, ImplCommandLine, ImplFrame, ImplRenderProcessHandler, Rect,
    RenderProcessHandler, RuntimeStyle, V8Context, WindowInfo, WrapApp, WrapBrowserProcessHandler,
    WrapRenderProcessHandler, rc::Rc as _, wrap_app, wrap_browser_process_handler,
    wrap_render_process_handler,
};

use crate::client::window_client;
use crate::panics::contain;

/// What `on_context_initialized` needs in order to open the window.
#[derive(Clone)]
pub struct WindowSpec {
    /// Le nom sous lequel la page désigne cette fenêtre. `"main"` pour
    /// celle que l'application ouvre en démarrant.
    pub name: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub url: String,
}

wrap_browser_process_handler! {
    struct EngineBrowserProcessHandler {
        spec: WindowSpec,
    }

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            let spec = self.spec.clone();
            contain("on_context_initialized", || open_window(&spec));
        }

        fn on_before_child_process_launch(&self, command_line: Option<&mut CommandLine>) {
            contain("on_before_child_process_launch", || {
                if let Some(command_line) = command_line {
                    forward_application_arguments(command_line);
                }
            });
        }
    }
}

/// Chromium builds each sub-process command line from scratch, so nothing the
/// application was launched with survives into the render process by itself.
/// Since the render process is where the application's Rust code runs, its
/// arguments have to travel with it: a switch means the same thing in every
/// process, or it means nothing.
///
/// Only the render process, though. The GPU, network and utility processes
/// run Chromium's code and none of the application's; an application switch
/// means nothing to them, and handing one to a process that parses its own
/// command line strictly is a way to lose that process for a reason nobody
/// will find.
fn forward_application_arguments(command_line: &CommandLine) {
    let kind = command_line.switch_value(Some(&CefString::from(PROCESS_TYPE_SWITCH)));
    if CefString::from(&kind).to_string() != RENDERER_PROCESS_TYPE {
        return;
    }
    // `args_os` rather than `args`: a path with a lone surrogate in it is a
    // reason to skip an argument, not to bring the application down.
    for argument in std::env::args_os().skip(1) {
        let Some(argument) = argument.to_str() else {
            eprintln!("rustx: skipped an argument that is not valid Unicode");
            continue;
        };
        command_line.append_argument(Some(&CefString::from(argument)));
    }
}

/// Chromium's own switch naming the kind of a sub-process.
const PROCESS_TYPE_SWITCH: &str = "type";
const RENDERER_PROCESS_TYPE: &str = "renderer";

wrap_render_process_handler! {
    struct EngineRenderProcessHandler;

    impl RenderProcessHandler {
        fn on_context_created(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            context: Option<&mut V8Context>,
        ) {
            contain("on_context_created", || {
                if let Some(context) = context {
                    crate::renderer::install(frame, context);
                }
            });
        }

        fn on_context_released(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            _context: Option<&mut V8Context>,
        ) {
            // A context going away ends what belonged to that context, and
            // nothing else. A sub-frame has one of its own — an iframe, a
            // preview pane, a widget being swapped out — and it comes and
            // goes while the application keeps running.
            //
            // The job queue belongs to the application, so it is closed
            // when the window closes (`LifeSpanHandler::on_before_close`),
            // not here. The DOM handles belong to the main frame's
            // document, so only the main frame's release makes them stale.
            //
            // The mailbox and the async runtime are a third thing again:
            // they belong to **this thread**, and this is the only hook
            // the render process gets. `on_before_close` runs in the
            // browser process, where there is no runtime and where
            // asking for the mailbox makes a new one just to close it —
            // so leaving it to that call left a future writing into the
            // page that came next, through a shared value found by its
            // name. `frame::open` takes the mailbox again when the next
            // context arrives, and the runtime starts again at the next
            // future.
            contain("on_context_released", || {
                let is_main = frame.is_some_and(|frame| frame.is_main() != 0);
                if is_main {
                    crate::dom::context_released();
                    crate::renderer::forget_context();
                    rustx_dom::state::close_remote();
                    crate::runtime::stop();
                }
            });
        }

        /// Ce que le processus de la fenêtre dit à la page.
        ///
        /// Sans ceci, `bridge::tell` écrit dans le vide : le message
        /// part, Chromium le livre, et personne ne le lit. Mesuré en
        /// lâchant un fichier sur une vraie fenêtre — les chemins
        /// n'arrivaient jamais, alors que les deux bouts étaient
        /// écrits.
        ///
        /// Rendre non-zéro dit à Chromium que le message est traité.
        fn on_process_message_received(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _source_process: cef::ProcessId,
            message: Option<&mut cef::ProcessMessage>,
        ) -> ::std::os::raw::c_int {
            let mut handled = 0;
            contain("on_process_message_received", || {
                if let Some(message) = message {
                    handled = i32::from(crate::bridge::told(message));
                }
            });
            handled
        }
    }
}

wrap_app! {
    struct EngineApp {
        browser_process: Option<BrowserProcessHandler>,
    }

    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            self.browser_process.clone()
        }

        fn render_process_handler(&self) -> Option<RenderProcessHandler> {
            Some(EngineRenderProcessHandler::new())
        }

        /// Chromium's own switches, before it reads them.
        fn on_before_command_line_processing(
            &self,
            _process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            contain("on_before_command_line_processing", || {
                let Some(command_line) = command_line else {
                    return;
                };
                if !software_rendering() {
                    return;
                }
                // A machine with no usable 3D driver — a virtual
                // machine, a remote desktop, a laptop whose driver has
                // given up — starts a GPU process that crashes, and
                // Chromium starts another, and another. The window then
                // produces no frames at all: on such a machine the log
                // says "Failed to create shared context for
                // virtualization" four times a second and the
                // application looks frozen with nothing wrong in it.
                //
                // These switches take the GPU out of the picture and
                // composite on the CPU, which is slower and always
                // works.
                for switch in [
                    "disable-gpu",
                    "disable-gpu-compositing",
                    "disable-gpu-sandbox",
                    "disable-software-rasterizer",
                ] {
                    command_line.append_switch(Some(&CefString::from(switch)));
                }
                command_line.append_switch_with_value(
                    Some(&CefString::from("use-angle")),
                    Some(&CefString::from("swiftshader")),
                );
            });
        }
    }
}

/// Should this window composite on the CPU?
///
/// Said by `RUSTX_SOFTWARE=1`, in the environment, because it has to be
/// true in every process of the application and the environment is what
/// every process already shares.
pub fn software_rendering() -> bool {
    std::env::var_os("RUSTX_SOFTWARE").is_some_and(|value| value != "0")
}

/// The application object for the process that owns the window.
pub fn engine_app(spec: WindowSpec) -> App {
    EngineApp::new(Some(EngineBrowserProcessHandler::new(spec)))
}

/// The application object for a sub-process. It has no window to open, but
/// it still carries the render process handler — which is the whole point.
pub fn child_app() -> App {
    EngineApp::new(None)
}

/// Ouvrir une fenêtre. Sur le thread de l'interface du processus
/// navigateur, et nulle part ailleurs : c'est la contrainte de CEF, et
/// c'est pourquoi une page passe par le pont pour en demander une.
pub(crate) fn open_window(spec: &WindowSpec) {
    // Le nom que `on_after_created` donnera à cette fenêtre.
    crate::window::expecting(&spec.name);

    let mut window_info =
        WindowInfo::default().set_as_popup(cef::sys::HWND(std::ptr::null_mut()), &spec.title);
    // Décalée d'autant de fenêtres qu'il y en a : une deuxième fenêtre
    // exactement sur la première a l'air de ne pas s'être ouverte.
    let step = 40 * crate::window::how_many_are_open() as i32;
    window_info.bounds = Rect {
        x: 80 + step,
        y: 80 + step,
        width: spec.width as i32,
        height: spec.height as i32,
    };
    // Alloy is the runtime style for a browser hosted in a window we own.
    // The default, Chrome style, brings the whole browser UI with it.
    window_info.runtime_style = RuntimeStyle::ALLOY;

    let mut client = window_client();
    cef::browser_host_create_browser(
        Some(&window_info),
        Some(&mut client),
        Some(&CefString::from(spec.url.as_str())),
        Some(&BrowserSettings::default()),
        None,
        None,
    );
}
