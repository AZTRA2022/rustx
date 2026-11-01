let mut ticks = 0;
let mut opens = 0;

fn Page() -> Element {
    return <div>
        <header>ticks <span id="header">{ticks}</span></header>
        {Panel()}
        <button id="leave" onClick={move |_| rustx::router::navigate("/pause".to_string())}>leave</button>
        <button id="another" onClick={move |_| {
            opens += 1;
            let _ = rustx::window::open(
                rustx::window::What::new("pause")
                    .titled("Monitor — pause")
                    .size(520, 400)
                    .at("/pause"),
            );
        }}>une autre fenêtre</button>
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
