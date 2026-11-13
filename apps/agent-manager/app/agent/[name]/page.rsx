// A second, real screen: `/agent/<name>`, the `[name]` segment
// handed straight to `Page` as its argument — no query string, no
// manual parsing. It is self-contained on purpose: it does not reach
// into the overview's live list, so a direct link to it (or a reload
// on it) always renders the same thing for the same name.

#[derive(rustx::ToValue, Clone)]
struct Point {
    at: f64,
    value: f64,
}

fn Page(name: String) -> Element {
    let (model, note) = profile_for(&name);
    let history = history_for(&name);

    return <div className="mx-auto max-w-3xl px-6 py-10">
        <button className="text-sm text-muted-foreground hover:text-foreground" onClick={move |_| rustx::router::navigate("/".to_string())}>
            ← back to the overview
        </button>

        <div className="mt-4 flex items-center gap-3">
            <h1 className="text-2xl font-semibold tracking-tight">{name.clone()}</h1>
            <Badge>{model.to_string()}</Badge>
        </div>
        <p className="mt-2 text-sm text-muted-foreground">{note.to_string()}</p>

        <Card className="mt-6 border-border bg-card">
            <CardHeader>
                <CardTitle className="text-sm uppercase tracking-wider text-muted-foreground">request history</CardTitle>
            </CardHeader>
            <CardContent>
                <recharts::LineChart width={560} height={220} data={history}>
                    <recharts::XAxis dataKey="at" stroke="#52525b" />
                    <recharts::YAxis stroke="#52525b" />
                    <recharts::Line type="monotone" dataKey="value" stroke="#818cf8" strokeWidth={2} isAnimationActive={false} />
                </recharts::LineChart>
            </CardContent>
        </Card>
    </div>;
}

fn profile_for(name: &str) -> (&'static str, &'static str) {
    match name {
        "scout" => ("haiku-class", "Watches incoming work and files the first pass."),
        "closer" => ("sonnet-class", "Takes what scout filed and finishes it."),
        "archivist" => ("haiku-class", "Idle by default — turned on only to catch up the archive."),
        _ => ("unknown-class", "No profile on file for this name."),
    }
}

fn history_for(name: &str) -> Vec<Point> {
    let seed: f64 = name.bytes().map(|byte| byte as f64).sum();
    (0..24)
        .map(|at| Point {
            at: at as f64,
            value: 3.0 + ((at as f64 * 0.5 + seed).sin().abs() * 7.0),
        })
        .collect()
}
