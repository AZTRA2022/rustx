//! The CEF application object.
//!
//! CEF will not let a browser be created straight after `initialize`: the
//! window has to be opened from `on_context_initialized`, once the browser
//! process context exists. This handler is where that happens, and it is also
//! where the render process handler will be attached.

use cef::{
    App, BrowserProcessHandler, BrowserSettings, CefString, ImplApp, ImplBrowserProcessHandler,
    Rect, RuntimeStyle, WindowInfo, WrapApp, WrapBrowserProcessHandler, rc::Rc as _, wrap_app,
    wrap_browser_process_handler,
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
    }
}

wrap_app! {
    struct EngineApp {
        browser_process: BrowserProcessHandler,
    }

    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(self.browser_process.clone())
        }
    }
}

/// The application object handed to `cef::initialize`.
pub fn engine_app(spec: WindowSpec) -> App {
    EngineApp::new(EngineBrowserProcessHandler::new(spec))
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
