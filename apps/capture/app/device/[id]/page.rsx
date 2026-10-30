fn Page(id: String) -> Element {
    let stream = keep(Stream::open(&id));
    let selected = store::<Selected>();

    return <div>
        <h1 id="preview">{stream.with(|stream| stream.device.clone())}</h1>
        <p id="selected-in-preview">{selected.get().0}</p>
        <button id="back" onClick={|_| { rustx_engine::router::back(); }}>back</button>
    </div>;
}
