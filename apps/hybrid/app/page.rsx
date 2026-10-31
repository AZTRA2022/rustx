let mut points = 0;

fn Page() -> Element {
    // The application's own data, for a component it did not write.
    let data: Vec<Point> = (0..=points)
        .map(|at| Point { at: at as f64, value: (at * at) as f64 })
        .collect();

    return <div>
        <h1 id="title">Hybrid</h1>

        <div id="chart">
            <LineChart width={320} height={160} data={data}>
                <XAxis dataKey="at" />
                <YAxis />
                <Line type="monotone" dataKey="value" stroke="#7f5af0" />
            </LineChart>
        </div>

        {Dialog()}

        <div id="panel-host"></div>

        <button id="add" onClick={move |_| points += 1}>add a point</button>
        <button id="leave" onClick={move |_| rustx::router::navigate("/gone".to_string())}>leave</button>
    </div>;
}

/// What shadcn/ui is made of: Radix for the behaviour, Tailwind for the
/// look. Both are written here — the tags are the library's components,
/// the classes are Tailwind's, and there is no JavaScript in this
/// application at all.
fn Dialog() -> Element {
    return <Root>
        <Trigger id="open-dialog" className="rounded-md bg-violet-500 px-3 py-2 font-medium text-white">
            about
        </Trigger>
        <Portal>
            <Overlay className="fixed inset-0 bg-black/60" />
            <Content
                id="about"
                className="fixed left-1/2 top-1/2 w-80 -translate-x-1/2 -translate-y-1/2 rounded-lg bg-zinc-900 p-6 shadow-xl"
            >
                <Title className="text-lg font-semibold">
                    One DOM, two owners
                </Title>
                <Description className="mt-2 text-sm text-zinc-400">
                    Radix for the behaviour, Tailwind for the look, both written in rsx.
                </Description>
                <Close id="close-dialog" className="mt-4 rounded-md bg-zinc-700 px-3 py-2">
                    close
                </Close>
            </Content>
        </Portal>
    </Root>;
}

fn Counter() -> Element {
    return <span id="counter-in-panel">{points}</span>;
}
