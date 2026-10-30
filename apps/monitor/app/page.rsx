fn Panel() -> Element {
    let ticks = store::<Ticks>();

    task(move |alive| {
        while alive.get() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = alive.post(|| store::<Ticks>().update(|ticks| ticks.0 += 1));
        }
    });

    return <p id="panel">{ticks.get().0}</p>;
}

fn Header() -> Element {
    let ticks = store::<Ticks>();
    return <span id="header">{ticks.get().0}</span>;
}
