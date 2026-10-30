fn Layout(children: Element) -> Element {
    let ticks = store::<Ticks>();

    task(move |alive| {
        while alive.get() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = alive.post(|| store::<Ticks>().update(|ticks| ticks.0 += 1));
        }
    });

    return <div>
        <header id="bar">uptime {ticks.get().0}</header>
        <section>{children}</section>
    </div>;
}
