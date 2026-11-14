// PLATFORM — what the layer offers, and a live demonstration of the
// one capability that is really here: reading a table.

let mut tables: Vec<Dataset> = Vec::new();
let mut status: String = String::new();

let mut looking_at: String = "users".to_string();

fn Page() -> Element {
    let table = tables.iter().find(|held| held.slug == looking_at).cloned();

    return <div key="platform" id="screen-platform" className="mx-auto max-w-[1000px] px-6 py-10">
        <div className="text-[10px] tracking-[0.16em] text-faint">PLATFORM</div>
        <h1 className="mt-2 text-[22px] font-semibold tracking-tight">What you get, and what it is</h1>
        <p className="mt-2.5 max-w-[600px] text-[12px] leading-[1.8] text-muted-foreground">
            Six capabilities. The last one is running in this window as you read this page; the
            first five are what the layer around it does, and are described rather than performed.
        </p>

        <div className="mt-7 grid grid-cols-3 gap-4">
            {capabilities().iter().map(|one| Capability(one.0.clone(), one.1.clone(), one.2)).collect::<Vec<_>>()}
        </div>

        <div className="mt-8 rounded-[6px] border border-border bg-card p-6">
            <div className="flex items-center">
                <span className="text-[10px] tracking-[0.16em] text-faint">LIVE — A TABLE, READ</span>
                <span className="ml-auto flex gap-1.5">
                    {tables.iter().map(|held| Pick(held.slug.clone(), held.name.clone())).collect::<Vec<_>>()}
                </span>
            </div>

            {match table {
                Some(held) => Preview(held),
                None => Empty(),
            }}
        </div>

        <p className="mt-5 text-[11px] text-muted-foreground">{status.clone()}</p>
    </div>;
}

fn Capability(name: String, text: String, here: bool) -> Element {
    let classes = format!(
        "rounded-[6px] border p-4 {}",
        if here { "border-[#d98a4a] bg-[#140e08]" } else { "border-border bg-card" }
    );
    return <div key={name.clone()} className={classes}>
        <div className="flex items-center gap-2">
            <span className="text-[12px] font-semibold">{name}</span>
            {if here {
                <Badge className="ml-auto border-[#d98a4a] bg-transparent text-[9px] text-warm">IN THIS WINDOW</Badge>
            } else {
                <Badge className="ml-auto border-border bg-transparent text-[9px] text-faint">DESCRIBED</Badge>
            }}
        </div>
        <p className="mt-2.5 text-[11px] leading-[1.7] text-muted-foreground">{text}</p>
    </div>;
}

fn Pick(slug: String, name: String) -> Element {
    let here = looking_at == slug;
    let classes = format!(
        "rounded-[4px] border px-2.5 py-[4px] text-[10px] {}",
        if here { "border-[#d98a4a] text-warm" } else { "border-border text-faint hover:text-foreground" }
    );
    let chosen = slug.clone();
    return <button key={slug.clone()} className={classes} onClick={move |_| looking_at = chosen.clone()}>
        {name}
    </button>;
}

fn Preview(table: Dataset) -> Element {
    let going = format!("/console/{}", table.slug);
    return <div key={table.slug.clone()}>
        <p className="mt-3 text-[12px] leading-[1.7] text-muted-foreground">{table.about.clone()}</p>
        <div className="mt-4 overflow-hidden rounded-[5px] border border-border">
            <Table>
                <TableHeader>
                    <TableRow>
                        {table.columns.iter().map(|column| Head(column.clone())).collect::<Vec<_>>()}
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {table.rows.iter().take(5).enumerate().map(|(at, row)| Row(at, row.clone())).collect::<Vec<_>>()}
                </TableBody>
            </Table>
        </div>
        <div className="mt-4 flex items-center gap-3">
            <button className="slab rounded-[5px] px-3.5 py-[7px] text-[11px]" onClick={move |_| rustx::router::navigate(going.clone())}>
                OPEN IT IN THE CONSOLE →
            </button>
            <span className="text-[10px] text-faint">
                {format!("showing 5 of {} row(s)", table.rows.len())}
            </span>
        </div>
    </div>;
}

fn Head(column: String) -> Element {
    return <TableHead key={column.clone()}>
        <span className="text-[10px] tracking-[0.1em] text-faint">{column}</span>
    </TableHead>;
}

fn Row(at: usize, row: Vec<String>) -> Element {
    return <TableRow key={at.to_string()}>
        {row.iter().enumerate().map(|(cell_at, cell)| Cell(at, cell_at, cell.clone())).collect::<Vec<_>>()}
    </TableRow>;
}

fn Cell(row_at: usize, cell_at: usize, value: String) -> Element {
    return <TableCell key={format!("{row_at}-{cell_at}")}>
        <span className="text-[11px]">{value}</span>
    </TableCell>;
}

fn Empty() -> Element {
    return <p className="mt-4 text-[11px] text-faint">
        The console holds nothing under that name. Drop a .csv on this window and it will.
    </p>;
}

/// Name, what it is, and whether it is really here.
fn capabilities() -> Vec<(String, String, bool)> {
    vec![
        ("Collectors".to_string(), "Agents at the edge, in the batch tier and in front of the warehouse, all speaking one shape.".to_string(), false),
        ("Noise floor".to_string(), "A score per signal and a line under it, tuned per source, so volume stops being the metric.".to_string(), false),
        ("Lineage".to_string(), "Every kept row knows which collector saw it and which rule let it through.".to_string(), false),
        ("Retention".to_string(), "A window per source, and a cold tier under it, because storage is where a data bill actually goes.".to_string(), false),
        ("Alerts".to_string(), "A rule over a column, evaluated as rows land, delivered where the team already reads.".to_string(), false),
        ("Console".to_string(), "Tables you can open, filter, sort, profile, edit and export. This one, in this window.".to_string(), true),
    ]
}
