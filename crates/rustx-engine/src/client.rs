//! The CEF client our windows are created with.
//!
//! CEF aborts when a browser is created without one, and the life span handler
//! is what tells the message loop that the last window is gone.

use cef::{
    Browser, CefString, Client, DisplayHandler, Frame, ImplBrowser, ImplBrowserHost, ImplClient,
    ImplDisplayHandler, ImplLifeSpanHandler, ImplRequest, ImplRequestHandler, LifeSpanHandler,
    LogSeverity, Request, RequestHandler, WrapClient, WrapDisplayHandler, WrapLifeSpanHandler,
    WrapRequestHandler, rc::Rc as _, wrap_client, wrap_display_handler, wrap_life_span_handler,
    wrap_request_handler,
};

use crate::guard;

wrap_life_span_handler! {
    struct WindowLifeSpan;

    impl LifeSpanHandler {
        /// The window exists now, so it can be dressed.
        ///
        /// The icon goes on here rather than in the window's
        /// description, because Chromium makes the window itself and
        /// the handle is the first thing it hands back.
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let handle = browser
                .and_then(|browser| browser.host())
                .map(|host| host.window_handle());
            if let Some(handle) = handle {
                crate::window::dress(handle.0.cast());
            }
        }

        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            // The window is going: stop accepting work from threads that
            // are still producing, then let the loop end.
            crate::frame::shutdown();
            cef::quit_message_loop();
        }
    }
}

wrap_request_handler! {
    struct LocalOnly;

    impl RequestHandler {
        /// Returning non-zero cancels the navigation.
        fn on_before_browse(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _user_gesture: ::std::os::raw::c_int,
            _is_redirect: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int {
            let url = request
                .map(|request| CefString::from(&request.url()).to_string())
                .unwrap_or_default();

            if guard::is_local(&url) {
                0
            } else {
                guard::refuse(&url);
                1
            }
        }
    }
}

wrap_display_handler! {
    struct Console;

    impl DisplayHandler {
        /// Everything the page writes to its console, and every error it
        /// did not catch.
        ///
        /// A page is JavaScript, and JavaScript fails inside the window,
        /// where nobody is looking. A developer whose bundle threw on
        /// its first line saw a blank screen and a complaint about a
        /// container nobody filled — the line that actually broke was in
        /// a console no terminal was reading. Returning 0 leaves the
        /// message in the browser's own console too.
        fn on_console_message(
            &self,
            _browser: Option<&mut Browser>,
            level: LogSeverity,
            message: Option<&CefString>,
            source: Option<&CefString>,
            line: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int {
            let message = message.map(ToString::to_string).unwrap_or_default();
            let source = source.map(ToString::to_string).unwrap_or_default();
            let severity = match level {
                LogSeverity::ERROR | LogSeverity::FATAL => "error",
                LogSeverity::WARNING => "warning",
                _ => "log",
            };
            // The file and line as the page sees them, which is what a
            // source map is read with.
            eprintln!("page {severity}: {message} ({source}:{line})");
            0
        }
    }
}

wrap_client! {
    struct WindowClient;

    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(WindowLifeSpan::new())
        }

        fn request_handler(&self) -> Option<RequestHandler> {
            Some(LocalOnly::new())
        }

        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(Console::new())
        }
    }
}

/// A client for a plain application window.
pub fn window_client() -> Client {
    WindowClient::new()
}
