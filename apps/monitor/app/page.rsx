let mut ticks = 0;

fn Page() -> Element {
    return <div>
        <header>ticks <span id="header">{ticks}</span></header>
        {Panel()}
        <button id="leave" onClick={move |_| rustx::router::navigate("/gone".to_string())}>leave</button>
    </div>;
}

fn Panel() -> Element {
    task(move |alive| {
        while alive.get() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = alive.post(|| ticks += 1);
        }
    });

    return <p id="panel">{ticks}</p>;
}

fn Header() -> Element {
    return <span id="header">{ticks}</span>;
}
