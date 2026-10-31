// The bar above every screen. Both values are written here, at the top
// of the file, which is what makes them the application's rather than
// one component's.
let mut ticks = 0;
let mut layout_starts = 0;

fn Layout(children: Element) -> Element {
    task(move |alive| {
        let _ = alive.post(|| layout_starts += 1);
        while alive.get() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = alive.post(|| ticks += 1);
        }
    });

    return <div>
        <header id="bar">uptime {ticks}</header>
        <section>{children}</section>
    </div>;
}
