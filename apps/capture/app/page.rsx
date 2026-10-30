fn Page() -> Element {
    let selected = store::<Selected>();

    // A component is called the way Rust calls a function. Each call is
    // an instance of its own, by the place it is written — the same rule
    // a `let mut` follows.
    return <ul id="devices">
        {Device("Webcam".to_string())}
        {Device("Micro".to_string())}
        {Device("Écran".to_string())}
        <li id="selected-in-list">{selected.get().0}</li>
    </ul>;
}

fn Device(name: String) -> Element {
    let opened = name.clone();
    return <li className="device" onClick={move |_| open(&opened)}>{name}</li>;
}
