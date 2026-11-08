//! The CEF client our windows are created with.
//!
//! CEF aborts when a browser is created without one, and the life span handler
//! is what tells the message loop that the last window is gone.

use cef::{
    Browser, CefString, CefStringList, Client, DisplayHandler, DragData, DragHandler,
    DragOperationsMask, Frame, ImplBrowser, ImplBrowserHost, ImplClient, ImplDisplayHandler,
    ImplDragData, ImplDragHandler, ImplLifeSpanHandler, ImplRequest, ImplRequestHandler,
    LifeSpanHandler, LogSeverity, Request, RequestHandler, WrapClient, WrapDisplayHandler,
    WrapDragHandler, WrapLifeSpanHandler, WrapRequestHandler, rc::Rc as _, wrap_client,
    wrap_display_handler, wrap_drag_handler, wrap_life_span_handler, wrap_request_handler,
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
            if let Some(browser) = browser.as_deref() {
                // Elle prend le nom qui l'attendait, pour que la page
                // puisse la refermer en le disant.
                crate::window::created(browser);
            }
            let handle = browser
                .and_then(|browser| browser.host())
                .map(|host| host.window_handle());
            if let Some(handle) = handle {
                crate::window::dress(handle.0.cast());
            }
        }

        fn on_before_close(&self, browser: Option<&mut Browser>) {
            // La dernière, ou pas : une application à deux fenêtres dont
            // on ferme la première ne s'arrête pas.
            if !crate::window::gone(browser.as_deref()) {
                return;
            }
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

wrap_drag_handler! {
    struct Dropped;

    impl DragHandler {
        /// Quelque chose entre dans la fenêtre, tenu par le curseur.
        ///
        /// C'est le seul moment où Chromium dit ce que c'est — son
        /// gestionnaire n'a pas d'événement pour le lâcher — et le seul
        /// endroit où le *chemin* d'un fichier est connu : ce que la
        /// page recevra, elle, est un `File` sans chemin.
        ///
        /// Les chemins partent donc vers le processus de la page, qui
        /// est le seul à savoir quand le lâcher a lieu. Rendre 0 laisse
        /// le dépôt se faire : ce handler écoute, il n'interdit rien.
        fn on_drag_enter(
            &self,
            browser: Option<&mut Browser>,
            drag_data: Option<&mut DragData>,
            _mask: DragOperationsMask,
        ) -> ::std::os::raw::c_int {
            let names = drag_data
                .map(|data| {
                    let mut list = CefStringList::new();
                    data.file_names(Some(&mut list));
                    list.into_iter().collect::<Vec<String>>()
                })
                .unwrap_or_default();

            let carried = crate::dropped::carried(&names);
            if !carried.is_empty() {
                if let Some(frame) = browser.and_then(|browser| browser.main_frame()) {
                    crate::bridge::tell(&frame, crate::dropped::WHAT, &carried);
                }
            }
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

        fn drag_handler(&self) -> Option<DragHandler> {
            Some(Dropped::new())
        }

        fn request_handler(&self) -> Option<RequestHandler> {
            Some(LocalOnly::new())
        }

        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(Console::new())
        }

        /// Ce que la page demande à la fenêtre.
        ///
        /// L'autre moitié du pont : le gestionnaire du processus de
        /// rendu écoute ce que la fenêtre dit à la page, et ceci écoute
        /// ce que la page demande à la fenêtre — une deuxième fenêtre,
        /// qui ne peut naître que de ce côté-ci.
        fn on_process_message_received(
            &self,
            browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _source_process: cef::ProcessId,
            message: Option<&mut cef::ProcessMessage>,
        ) -> ::std::os::raw::c_int {
            let mut handled = 0;
            crate::panics::contain("on_process_message_received", || {
                if let Some(message) = message {
                    handled = i32::from(crate::window::asked(message, browser.as_deref()));
                }
            });
            handled
        }
    }
}

/// A client for a plain application window.
pub fn window_client() -> Client {
    WindowClient::new()
}
