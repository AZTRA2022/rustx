fn Page() -> Element {
    let points = store::<Points>();

    return <div>
        <h1 id="title">Hybrid</h1>
        <js::Chart points={points.get().0 as f64} />
        <div id="panel-host"></div>
        <button id="add" onClick={move |_| add_point()}>add a point</button>
        <button id="leave" onClick={move |_| leave()}>leave</button>
    </div>;
}
