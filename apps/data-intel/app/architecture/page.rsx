// ARCHITECTURE — what the layer is made of, drawn rather than
// described, and honest about which parts exist in this window.

let mut tables: Vec<Dataset> = Vec::new();

let mut stage: usize = 1;

fn Page() -> Element {
    let all = stages();
    let here = stage.min(all.len() - 1);
    let rows: usize = tables.iter().map(|table| table.rows.len()).sum();

    return <div key="architecture" id="screen-architecture" className="mx-auto max-w-[1000px] px-6 py-10">
        <div className="text-[10px] tracking-[0.16em] text-faint">ARCHITECTURE</div>
        <h1 className="mt-2 text-[22px] font-semibold tracking-tight">Four layers, one direction</h1>
        <p className="mt-2.5 max-w-[580px] text-[12px] leading-[1.8] text-muted-foreground">
            Signals arrive, are reduced, are kept, and are read. Nothing flows the other way. The
            fourth layer is the one that ships in this window; the first three are described here
            and are not pretended at.
        </p>

        <div className="mt-8 flex items-stretch gap-3">
            {all.iter().enumerate().map(|(at, one)| Block(at, one.0.clone(), one.1.clone(), at == here)).collect::<Vec<_>>()}
        </div>

        <div className="mt-6 rounded-[6px] border border-border bg-card p-6">
            <div className="text-[10px] tracking-[0.16em] text-faint">
                {format!("LAYER {} — {}", here + 1, all[here].0.clone().to_uppercase())}
            </div>
            <p className="mt-3 max-w-[700px] text-[13px] leading-[1.85]">{all[here].2.clone()}</p>
            <div className="mt-5 flex gap-2">
                <button className="slab rounded-[5px] px-3.5 py-[7px] text-[11px]" disabled={here == 0} onClick={move |_| stage = stage.saturating_sub(1)}>
                    ← PREVIOUS
                </button>
                <button className="slab rounded-[5px] px-3.5 py-[7px] text-[11px]" disabled={here + 1 >= all.len()} onClick={move |_| stage += 1}>
                    NEXT →
                </button>
                <button className="glossy ml-auto rounded-[5px] px-3.5 py-[7px] text-[11px]" onClick={move |_| rustx::router::navigate("/console".to_string())}>
                    {format!("OPEN THE CONSOLE — {rows} ROWS")}
                </button>
            </div>
        </div>

        <div className="mt-7 grid grid-cols-3 gap-4">
            {Note("WHAT IS REAL HERE".to_string(), "The read layer: tables in memory, a file dropped on the window with its real path, a CSV written back to disk.".to_string(), "/docs/the-console".to_string())}
            {Note("WHAT IS DESCRIBED".to_string(), "Ingest, reduction and storage. There is no driver and no wire protocol in this application, and no page claims one.".to_string(), "/docs/getting-data-in".to_string())}
            {Note("WHY IT IS BUILT SO".to_string(), "The part a demonstration can get right is the part a reader can check. Everything checkable is here; the rest is named.".to_string(), "/docs/architecture".to_string())}
        </div>
    </div>;
}

fn Block(at: usize, name: String, under: String, here: bool) -> Element {
    let classes = format!(
        "flex-1 rounded-[6px] border p-4 text-left {}",
        if here { "border-[#d98a4a] bg-[#140e08]" } else { "border-border bg-card hover:border-[#3a3a3a]" }
    );
    return <button key={name.clone()} className={classes} onClick={move |_| stage = at}>
        <span className="block text-[9px] tracking-[0.16em] text-faint">{format!("0{}", at + 1)}</span>
        <span className="mt-2 block text-[14px] font-semibold">{name}</span>
        <span className="mt-1.5 block text-[11px] leading-[1.6] text-muted-foreground">{under}</span>
    </button>;
}

fn Note(name: String, text: String, route: String) -> Element {
    let going = route.clone();
    return <button
        key={name.clone()}
        className="rounded-[6px] border border-border bg-card p-4 text-left hover:border-[#3a3a3a]"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="block text-[9px] tracking-[0.16em] text-faint">{name}</span>
        <span className="mt-2.5 block text-[11px] leading-[1.7] text-muted-foreground">{text}</span>
        <span className="mt-3 block text-[9px] tracking-[0.12em] text-warm">READ ON →</span>
    </button>;
}

fn stages() -> Vec<(String, String, String)> {
    vec![
        (
            "Ingest".to_string(),
            "Everything that reaches the layer".to_string(),
            "Signals arrive from edges, batches and warehouses, and the first thing that happens to them is that they stop being different shapes. One key, one timestamp, one source. This layer is described in this application and not implemented by it: there is no collector in this window and no page says there is.".to_string(),
        ),
        (
            "Reduce".to_string(),
            "The noise floor, and what clears it".to_string(),
            "A row that tells you nothing costs as much to keep as one that does. Reduction is a score per signal and a floor under it; what clears the floor is kept with the score attached, which is what the signals table in the console is a sample of. The scoring itself lives in the layer, not here.".to_string(),
        ),
        (
            "Keep".to_string(),
            "Columnar, cheap, and boring".to_string(),
            "Storage is the layer nobody should have to think about, which is why it is the layer with the fewest ideas in it. Columns, compression, and a retention window per source. This application holds its tables in memory for as long as the window is open, and exports to a plain CSV when you want them to outlive it.".to_string(),
        ),
        (
            "Read".to_string(),
            "The console, which is in this window".to_string(),
            "A table is something a person can open, filter, sort, profile and take away. That is the whole of this layer and it is the one that actually ships here: /console lists every table the application holds, /console/<name> is one of them, a .csv dropped on the window becomes a table through rustx::dropped, and EXPORT writes a real file through the system's own save dialog.".to_string(),
        ),
    ]
}
