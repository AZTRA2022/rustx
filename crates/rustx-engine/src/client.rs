//! The CEF client our windows are created with.
//!
//! CEF aborts when a browser is created without one, and the life span handler
//! is what tells the message loop that the last window is gone.

use cef::{
    Browser, CefString, Client, Frame, ImplClient, ImplLifeSpanHandler, ImplRequest,
    ImplRequestHandler, LifeSpanHandler, Request, RequestHandler, WrapClient, WrapLifeSpanHandler,
    WrapRequestHandler, rc::Rc as _, wrap_client, wrap_life_span_handler, wrap_request_handler,
};

use crate::guard;

wrap_life_span_handler! {
    struct WindowLifeSpan;

    impl LifeSpanHandler {
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

wrap_client! {
    struct WindowClient;

    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(WindowLifeSpan::new())
        }

        fn request_handler(&self) -> Option<RequestHandler> {
            Some(LocalOnly::new())
        }
    }
}

/// A client for a plain application window.
pub fn window_client() -> Client {
    WindowClient::new()
}
