// SOLUTIONS — who the layer is for, and what each of them opens the
// console to look at.

let mut tables: Vec<Dataset> = Vec::new();

let mut chosen: usize = 0;

fn Page() -> Element {
    let all = solutions();
    let here = chosen.min(all.len() - 1);
    let one = all[here].clone();

    return <div key="solutions" id="screen-solutions" className="mx-auto max-w-[1000px] px-6 py-10">
        <div className="text-[10px] tracking-[0.16em] text-faint">SOLUTIONS</div>
        <h1 className="mt-2 text-[22px] font-semibold tracking-tight">Four rooms with the same problem</h1>
        <p className="mt-2.5 max-w-[600px] text-[12px] leading-[1.8] text-muted-foreground">
            Every one of them is drowning in rows that mean nothing and short of the three that do.
            Pick one and the page says what it looks like from there.
        </p>

        <div className="mt-7 grid grid-cols-4 gap-3">
            {all.iter().enumerate().map(|(at, item)| Tab(at, item.0.clone(), at == here)).collect::<Vec<_>>()}
        </div>

        <div className="mt-5 grid grid-cols-[1.3fr_1fr] gap-4">
            <div className="rounded-[6px] border border-border bg-card p-6">
                <div className="text-[10px] tracking-[0.16em] text-faint">{one.0.clone().to_uppercase()}</div>
                <p className="mt-3 text-[13px] leading-[1.85]">{one.1.clone()}</p>
                <div className="mt-5 flex flex-col gap-2">
                    {one.2.iter().map(|line| Point(line.clone())).collect::<Vec<_>>()}
                </div>
            </div>

            <div className="rounded-[6px] border border-border bg-card p-6">
                <div className="text-[10px] tracking-[0.16em] text-faint">WHAT THEY OPEN FIRST</div>
                {match tables.iter().find(|table| table.slug == one.3).cloned() {
                    Some(table) => Shortcut(table),
                    None => Nothing(one.3.clone()),
                }}
                <p className="mt-5 text-[11px] leading-[1.7] text-muted-foreground">
                    Every table named on this page is one the console actually holds, so the link
                    below is a screen and not a promise.
                </p>
            </div>
        </div>
    </div>;
}

fn Tab(at: usize, name: String, here: bool) -> Element {
    let classes = format!(
        "rounded-[6px] border px-4 py-3 text-left {}",
        if here { "border-[#d98a4a] bg-[#140e08]" } else { "border-border bg-card hover:border-[#3a3a3a]" }
    );
    return <button key={name.clone()} className={classes} onClick={move |_| chosen = at}>
        <span className="block text-[9px] tracking-[0.14em] text-faint">{format!("0{}", at + 1)}</span>
        <span className="mt-1.5 block text-[12px] font-semibold">{name}</span>
    </button>;
}

fn Point(text: String) -> Element {
    return <div key={text.clone()} className="flex gap-2.5 text-[11px] leading-[1.65] text-muted-foreground">
        <span className="text-warm">—</span>
        <span>{text}</span>
    </div>;
}

fn Shortcut(table: Dataset) -> Element {
    let going = format!("/console/{}", table.slug);
    return <div key={table.slug.clone()} className="mt-3">
        <div className="text-[15px] font-semibold">{table.name.clone()}</div>
        <p className="mt-1.5 text-[11px] leading-[1.7] text-muted-foreground">{table.about.clone()}</p>
        <div className="mt-2.5 text-[10px] text-faint">
            {format!("{} row(s) × {} column(s)", table.rows.len(), table.columns.len())}
        </div>
        <button className="slab mt-4 rounded-[5px] px-3.5 py-[7px] text-[11px]" onClick={move |_| rustx::router::navigate(going.clone())}>
            OPEN IT →
        </button>
    </div>;
}

fn Nothing(slug: String) -> Element {
    return <div className="mt-3">
        <p className="text-[11px] text-faint">
            {format!("The console no longer holds a table called “{slug}”. It was dropped, or never imported.")}
        </p>
        <button className="slab mt-4 rounded-[5px] px-3.5 py-[7px] text-[11px]" onClick={move |_| rustx::router::navigate("/console".to_string())}>
            OPEN THE CONSOLE
        </button>
    </div>;
}

/// Name, the paragraph, the three lines under it, and the table that
/// room opens first.
fn solutions() -> Vec<(String, String, Vec<String>, String)> {
    vec![
        (
            "Platform teams".to_string(),
            "The team that gets paged owns a hundred dashboards and reads two of them. What they need is not another panel: it is a floor under the noise and a table of what cleared it, in an order they can sort.".to_string(),
            vec![
                "One row per event that mattered, with the score that let it through".to_string(),
                "Sort by score, filter by source, export what the incident review needs".to_string(),
                "No new query language to learn on the day it breaks".to_string(),
            ],
            "signals".to_string(),
        ),
        (
            "Finance".to_string(),
            "Charges, states and the gap between them. Finance does not want a stream; it wants the same table every month, in the same shape, and a file it can hand to someone else.".to_string(),
            vec![
                "Filter to the state that is holding the month open".to_string(),
                "Profile a column to see where the volume actually sits".to_string(),
                "Export a real CSV through the system's own save dialog".to_string(),
            ],
            "orders".to_string(),
        ),
        (
            "Growth".to_string(),
            "Who signed up, what plan they sit on, and where. Three columns and a count is most of what a growth question is, and it is the question a dashboard is worst at answering.".to_string(),
            vec![
                "Profile the plan column to see the shape of the base".to_string(),
                "Filter by region without writing a clause".to_string(),
                "Edit a cell when the source got it wrong".to_string(),
            ],
            "users".to_string(),
        ),
        (
            "Anyone with a file".to_string(),
            "The honest fourth case. A CSV on a laptop, and no appetite for a database to read it. Drop it on this window and it is a table, with the same filtering, sorting, profiling and export as everything else.".to_string(),
            vec![
                "Dropped files are read from their real path, not uploaded".to_string(),
                "Nothing leaves the machine unless you save it there yourself".to_string(),
                "The table lives as long as the window does".to_string(),
            ],
            "users".to_string(),
        ),
    ]
}
