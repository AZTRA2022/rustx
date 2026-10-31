let mut points = 0;

fn Page() -> Element {
    // The application's own data, for a component it did not write.
    let data: Vec<Point> = (0..=points)
        .map(|at| Point { at: at as f64, value: (at * at) as f64 })
        .collect();

    return <div>
        <h1 id="title">Hybrid</h1>

        <div id="chart">
            <js::recharts::LineChart width={320} height={160} data={data}>
                <js::recharts::XAxis dataKey="at" />
                <js::recharts::YAxis />
                <js::recharts::Line type="monotone" dataKey="value" stroke="#7f5af0" />
            </js::recharts::LineChart>
        </div>

        <div id="panel-host"></div>

        <button id="add" onClick={move |_| points += 1}>add a point</button>
        <button id="leave" onClick={move |_| rustx::router::navigate("/gone".to_string())}>leave</button>
    </div>;
}

fn Counter() -> Element {
    return <span id="counter-in-panel">{points}</span>;
}
