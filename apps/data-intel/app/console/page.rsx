// THE CONSOLE — what GET STARTED opens, and the part of this
// application that does work.
//
// Every table it holds is listed here with its shape, and every one of
// them has its own address at `/console/<name>`. The two ways data
// gets in are real: drop a `.csv` on the window, or open one with the
// system's own dialog. The chart is `recharts`, off npm, fed the real
// row counts.

let mut tables: Vec<Dataset> = Vec::new();
let mut status: String = String::new();
let mut imported: u32 = 0;

/// A bar of the overview chart.
#[derive(rustx::ToValue, Clone)]
struct Bar {
    table: String,
    rows: f64,
    columns: f64,
}

fn Page() -> Element {
    let bars: Vec<Bar> = tables
        .iter()
        .map(|table| Bar {
            table: table.name.clone(),
            rows: table.rows.len() as f64,
            columns: table.columns.len() as f64,
        })
        .collect();
    let rows: usize = tables.iter().map(|table| table.rows.len()).sum();
    let cells: usize = tables
        .iter()
        .map(|table| table.rows.len() * table.columns.len())
        .sum();

    return <div key="console" id="screen-console" className="mx-auto max-w-[1060px] px-6 py-9">
        <div className="flex items-end">
            <div>
                <h1 className="text-[18px] font-semibold tracking-tight">Console</h1>
                <p className="mt-1.5 text-[12px] text-muted-foreground">
                    {format!("{} table(s), {rows} row(s), {cells} cell(s) under management.", tables.len())}
                </p>
            </div>
            <div className="ml-auto flex items-center gap-2">
                <button className="slab rounded-[5px] px-3.5 py-[7px] text-[11px]" onClick={move |_| open_one()}>
                    IMPORT A CSV
                </button>
                <button className="slab rounded-[5px] px-3.5 py-[7px] text-[11px]" onClick={move |_| add_blank()}>
                    NEW TABLE
                </button>
            </div>
        </div>

        <div className="mt-5 rounded-[6px] border border-border bg-card p-5">
            <div className="text-[10px] tracking-[0.16em] text-faint">SHAPE OF EVERY TABLE</div>
            <div className="mt-4">
                <recharts::BarChart width={1000} height={170} data={bars}>
                    <recharts::CartesianGrid stroke="#1a1a1a" vertical={false} />
                    <recharts::XAxis dataKey="table" stroke="#262626" tickLine={false} />
                    <recharts::YAxis stroke="#262626" tickLine={false} allowDecimals={false} width={32} />
                    <recharts::Tooltip isAnimationActive={false} cursor={false} />
                    <recharts::Bar dataKey="rows" name="Rows" fill="#d98a4a" isAnimationActive={false} />
                    <recharts::Bar dataKey="columns" name="Columns" fill="#9ab6d6" isAnimationActive={false} />
                </recharts::BarChart>
            </div>
        </div>

        <div id="tables" className="mt-5 grid grid-cols-3 gap-4">
            {tables.iter().map(|table| Card(table.clone())).collect::<Vec<_>>()}
        </div>

        {if tables.is_empty() {
            <p className="mt-6 text-[12px] text-faint">
                No tables. Drop a .csv on this window, or press IMPORT A CSV.
            </p>
        } else {
            <span></span>
        }}

        <p className="mt-6 text-[11px] text-muted-foreground">
            {if status.is_empty() {
                "Drop a .csv anywhere on this window and it becomes a table — the window is told the file's real path, which is the only place one is known.".to_string()
            } else {
                status.clone()
            }}
        </p>
    </div>;
}

fn Card(table: Dataset) -> Element {
    let going = format!("/console/{}", table.slug);
    let slug = table.slug.clone();

    return <div key={table.slug.clone()} className="rounded-[6px] border border-border bg-card p-4">
        <div className="flex items-center gap-2">
            <span className="text-[13px] font-semibold">{table.name.clone()}</span>
            <Badge className="ml-auto border-border bg-[#141414] text-[9px] text-muted-foreground">
                {format!("{} × {}", table.rows.len(), table.columns.len())}
            </Badge>
        </div>
        <p className="mt-2 text-[11px] leading-[1.65] text-muted-foreground">{table.about.clone()}</p>
        <div className="mt-3 flex flex-wrap gap-1.5">
            {table.columns.iter().map(|column| Column(table.slug.clone(), column.clone())).collect::<Vec<_>>()}
        </div>
        <div className="mt-4 flex gap-2">
            <button
                className="slab rounded-[5px] px-3 py-[6px] text-[10px]"
                onClick={move |_| rustx::router::navigate(going.clone())}
            >
                OPEN →
            </button>
            <button
                className="rounded-[5px] border border-border px-3 py-[6px] text-[10px] text-muted-foreground hover:text-foreground"
                onClick={move |_| drop_table(slug.clone())}
            >
                DROP
            </button>
        </div>
    </div>;
}

fn Column(slug: String, column: String) -> Element {
    return <span key={format!("{slug}-{column}")} className="rounded-[3px] border border-border px-1.5 py-[2px] text-[9px] text-faint">
        {column}
    </span>;
}

// ---- what the buttons do ---------------------------------------------

fn open_one() {
    let Ok(paths) = rustx::dialog::open(
        rustx::dialog::Ask::new("Import a CSV table").filter("Comma separated", ["csv"]),
    ) else {
        status = "the import was cancelled".to_string();
        return;
    };
    let Some(path) = paths.first() else {
        status = "the import was cancelled".to_string();
        return;
    };

    let Ok(text) = std::fs::read_to_string(path) else {
        status = format!("{} could not be read", path.display());
        return;
    };
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        status = format!("{} is empty", path.display());
        return;
    };
    let columns: Vec<String> = header.split(',').map(|cell| cell.trim().to_string()).collect();
    let rows: Vec<Vec<String>> = lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split(',').map(|cell| cell.trim().to_string()).collect())
        .collect();

    let name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "imported".to_string());
    let mut slug = slugify(&name);
    let mut all = tables.clone();
    while all.iter().any(|table| table.slug == slug) {
        slug = format!("{slug}-2");
    }
    let count = rows.len();
    all.push(Dataset {
        slug,
        name: name.clone(),
        about: format!("Read from {}.", path.display()),
        columns,
        rows,
    });
    tables = all;
    imported += 1;
    status = format!("{name} — {count} row(s) read from {}", path.display());
}

/// An empty table, so the console is never a screen you cannot use
/// because you have no file to hand.
fn add_blank() {
    let mut all = tables.clone();
    let mut slug = format!("table-{}", all.len() + 1);
    while all.iter().any(|table| table.slug == slug) {
        slug = format!("{slug}-2");
    }
    all.push(Dataset {
        slug: slug.clone(),
        name: slug.clone(),
        about: "Made in the console. Add rows on its own screen.".to_string(),
        columns: vec!["key".to_string(), "value".to_string()],
        rows: Vec::new(),
    });
    tables = all;
    status = format!("{slug} is ready — open it to add rows");
}

fn drop_table(slug: String) {
    let mut all = tables.clone();
    all.retain(|table| table.slug != slug);
    tables = all;
    status = format!("{slug} dropped");
}
