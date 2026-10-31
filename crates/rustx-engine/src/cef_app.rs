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
            contain("on_context_released", || {
                let is_main = frame.is_some_and(|frame| frame.is_main() != 0);
                if is_main {
                    crate::dom::context_released();
                }
            });
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
    }
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

fn open_window(spec: &WindowSpec) {
    let mut window_info =
        WindowInfo::default().set_as_popup(cef::sys::HWND(std::ptr::null_mut()), &spec.title);
    window_info.bounds = Rect {
        x: 80,
        y: 80,
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
