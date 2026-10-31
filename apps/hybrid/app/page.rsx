let mut points = 0;

fn Page() -> Element {
    return <div>
        <h1 id="title">Hybrid</h1>
        <js::Chart points={points as f64} />
        <div id="panel-host"></div>
        <button id="add" onClick={move |_| points += 1}>add a point</button>
        <button id="leave" onClick={move |_| rustx::router::navigate("/gone".to_string())}>leave</button>
    </div>;
}

fn Counter() -> Element {
    return <span id="counter-in-panel">{points}</span>;
}
