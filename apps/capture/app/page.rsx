fn Page() -> Element {
    let selected = store::<Selected>();

    return <ul id="devices">
        <Device name={"Webcam".to_string()} />
        <Device name={"Micro".to_string()} />
        <Device name={"Écran".to_string()} />
        <li id="selected-in-list">{selected.get().0}</li>
        {Device("astaroth".to_string())}
    </ul>;
}

fn Device(name: String) -> Element {
    let opened = name.clone();
    return <li className="device text-red-500" onClick={move |_| open(&opened)}>{name}</li>;
}
