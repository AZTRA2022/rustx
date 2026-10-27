//! The CEF application object.
//!
//! One object, two jobs, because CEF hands the same `App` to both halves of
//! the relaunch. In the browser process it opens the window; in the render
//! process it installs the application's Rust functions on the JavaScript
//! global. CEF will not let a browser be created straight after `initialize`,
//! so the window is opened from `on_context_initialized`.

use cef::{
    App, Browser, BrowserProcessHandler, BrowserSettings, CefString, CommandLine, Frame, ImplApp,
    ImplCommandLine,
    ImplBrowserProcessHandler, ImplRenderProcessHandler, Rect, RenderProcessHandler, RuntimeStyle,
    V8Context, WindowInfo, WrapApp, WrapBrowserProcessHandler, WrapRenderProcessHandler,
    rc::Rc as _, wrap_app, wrap_browser_process_handler, wrap_render_process_handler,
};

use crate::client::window_client;

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
            open_window(&self.spec);
        }

        fn on_before_child_process_launch(&self, command_line: Option<&mut CommandLine>) {
            if let Some(command_line) = command_line {
                forward_application_arguments(command_line);
            }
        }
    }
}

/// Chromium builds each sub-process command line from scratch, so nothing the
/// application was launched with survives into the render process by itself.
/// Since the render process is where the application's Rust code runs, its
/// arguments have to travel with it: a switch means the same thing in every
/// process, or it means nothing.
fn forward_application_arguments(command_line: &CommandLine) {
    for argument in std::env::args().skip(1) {
        command_line.append_argument(Some(&CefString::from(argument.as_str())));
    }
}

wrap_render_process_handler! {
    struct EngineRenderProcessHandler;

    impl RenderProcessHandler {
        fn on_context_created(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            context: Option<&mut V8Context>,
        ) {
            if let Some(context) = context {
                crate::renderer::install(frame, context);
            }
        }

        fn on_context_released(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            _context: Option<&mut V8Context>,
        ) {
            // Every DOM handle made in that context is stale from here on,
            // and a worker thread still posting work must be told to stop.
            crate::dom::context_released();
            crate::frame::shutdown();
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
