//! The CEF client our windows are created with.
//!
//! CEF aborts when a browser is created without one, and the life span handler
//! is what tells the message loop that the last window is gone.

use cef::{
    Browser, Client, ImplClient, ImplLifeSpanHandler, LifeSpanHandler, WrapClient,
    WrapLifeSpanHandler, rc::Rc as _, wrap_client, wrap_life_span_handler,
};

wrap_life_span_handler! {
    struct WindowLifeSpan;

    impl LifeSpanHandler {
        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            cef::quit_message_loop();
        }
    }
}

wrap_client! {
    struct WindowClient;

    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(WindowLifeSpan::new())
        }
    }
}

/// A client for a plain application window.
pub fn window_client() -> Client {
    WindowClient::new()
}
