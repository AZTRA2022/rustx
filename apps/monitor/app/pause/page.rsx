// Where the monitor's "leave" button goes.
//
// A real screen rather than a path nobody declared: what is proved is
// that leaving the page stops the thread the panel kept, and that is
// true of any screen. A path that does not exist proved it while showing
// "This page does not exist." to anyone who opened the application.

fn Page() -> Element {
    return <div>
        <p id="paused">le moniteur est arrêté</p>
        <button id="back" onClick={move |_| rustx::router::navigate("/".to_string())}>revenir</button>
    </div>;
}
