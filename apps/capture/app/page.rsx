// What the user picked, and how many streams the previews have closed.
// Both are read on the other screen too: a value written at the top of
// a page is the application's, by the name it was written with.
let mut selected = String::new();
let mut closed_streams = 0;

fn Page() -> Element {
    // A component is called the way Rust calls a function. Each call is
    // an instance of its own, by the place it is written — the same rule
    // a `let mut` follows.
    return <ul id="devices">
        {Device("Webcam".to_string())}
        {Device("Micro".to_string())}
        {Device("Écran".to_string())}
        <li id="selected-in-list">{selected}</li>
    </ul>;
}

fn Device(name: String) -> Element {
    let opened = name.clone();
    return <li className="device" onClick={move |_| {
        selected = opened.clone();
        rustx::router::navigate(format!("/device/{opened}"));
    }}>{name}</li>;
}
