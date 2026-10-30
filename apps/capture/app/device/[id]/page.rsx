fn Page(id: String) -> Element {
    // The stream is kept under the device's own key, not just on this
    // page. `/device/Webcam` and `/device/Micro` are the same route and
    // so the same component: without the key the first stream would be
    // kept and the preview would show a device the path no longer names.
    let stream = with_key(id.clone(), || keep(Stream::open(&id)));
    let selected = store::<Selected>();

    // Somewhere else to go that is the same route — which is what makes
    // this a parameter change rather than a navigation.
    let other = if id == "Micro" { "Webcam" } else { "Micro" };
    let elsewhere = format!("/device/{other}");

    return <div>
        <h1 id="preview">{stream.with(|stream| stream.device.clone())}</h1>
        <p id="selected-in-preview">{selected.get().0}</p>
        <button id="other" onClick={move |_| { rustx::router::navigate(elsewhere.clone()); }}>other</button>
        <button id="back" onClick={|_| { rustx::router::back(); }}>back</button>
    </div>;
}
