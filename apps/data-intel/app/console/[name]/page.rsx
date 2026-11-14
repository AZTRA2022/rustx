// ONE TABLE — its rows, and what can be asked of them.
//
// Filtering, sorting and profiling are done over the live table, so a
// file dropped on the window a moment ago behaves here like anything
// else. EXPORT writes a real CSV where the system's own dialog puts
// it; COPY puts the same text on the real clipboard.

let mut tables: Vec<Dataset> = Vec::new();
let mut status: String = String::new();

let mut needle: String = String::new();
let mut sort_by: usize = 0;
let mut descending: bool = false;
let mut profile_of: usize = 0;

/// A bar of the column profile.
#[derive(rustx::ToValue, Clone)]
struct Count {
    value: String,
    rows: f64,
}

fn Page(name: String) -> Element {
    let found = tables.iter().find(|table| table.slug == name).cloned();

    return <div key={format!("table-{name}")} id="screen-table" className="mx-auto max-w-[1060px] px-6 py-9">
        <div className="flex items-center gap-2 text-[11px] text-muted-foreground">
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/console".to_string())}>
                Console
            </button>
            <span className="text-faint">/</span>
            <span className="text-foreground">
                {found.as_ref().map(|table| table.name.clone()).unwrap_or_else(|| "unknown".to_string())}
            </span>
        </div>

        {match found {
            Some(table) => Body(table),
            None => Missing(name.clone()),
        }}
    </div>;
}

fn Body(table: Dataset) -> Element {
    let shown = visible(&table);
    let column = profile_of.min(table.columns.len().saturating_sub(1));
    let counts = profile(&table, column);
    let slug = table.slug.clone();
    let to_export = table.clone();
    let to_copy = table.clone();

    return <div key={table.slug.clone()}>
        <div className="mt-3 flex items-end">
            <div>
                <h1 className="text-[18px] font-semibold tracking-tight">{table.name.clone()}</h1>
                <p className="mt-1.5 max-w-[520px] text-[12px] leading-[1.65] text-muted-foreground">{table.about.clone()}</p>
            </div>
            <div className="ml-auto flex items-center gap-2">
                <span className="flex items-center gap-2 rounded-[5px] border border-border bg-card px-2.5 py-[6px]">
                    <input
                        id="filter"
                        className="w-44 bg-transparent text-[11px] outline-none placeholder:text-faint"
                        placeholder="Filter any cell"
                        value={needle.clone()}
                        onInput={move |event| needle = event.value}
                    />
                </span>
                <button className="slab rounded-[5px] px-3 py-[6px] text-[10px]" onClick={move |_| copy_it(to_copy.clone())}>
                    COPY
                </button>
                <button className="slab rounded-[5px] px-3 py-[6px] text-[10px]" onClick={move |_| export_it(to_export.clone())}>
                    EXPORT
                </button>
                <button className="slab rounded-[5px] px-3 py-[6px] text-[10px]" onClick={move |_| add_row(slug.clone())}>
                    ADD ROW
                </button>
            </div>
        </div>

        <div className="mt-5 grid grid-cols-4 gap-3">
            {Fact("ROWS".to_string(), table.rows.len().to_string())}
            {Fact("SHOWN".to_string(), shown.len().to_string())}
            {Fact("COLUMNS".to_string(), table.columns.len().to_string())}
            {Fact("DISTINCT".to_string(), counts.len().to_string())}
        </div>

        <div className="mt-5 overflow-hidden rounded-[6px] border border-border bg-card">
            <Table>
                <TableHeader>
                    <TableRow>
                        {table.columns.iter().enumerate().map(|(at, column)| Head(at, column.clone())).collect::<Vec<_>>()}
                        <TableHead></TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {shown.iter().map(|row| Row(table.slug.clone(), row.0, row.1.clone())).collect::<Vec<_>>()}
                </TableBody>
            </Table>
            {if shown.is_empty() {
                <div className="px-4 py-6 text-[11px] text-faint">
                    {if table.rows.is_empty() { "This table has no rows yet." } else { "Nothing matches that filter." }}
                </div>
            } else {
                <span></span>
            }}
        </div>

        <div className="mt-5 rounded-[6px] border border-border bg-card p-5">
            <div className="flex items-center">
                <span className="text-[10px] tracking-[0.16em] text-faint">COLUMN PROFILE</span>
                <span className="ml-auto flex gap-1.5">
                    {table.columns.iter().enumerate().map(|(at, column)| Pick(at, column.clone())).collect::<Vec<_>>()}
                </span>
            </div>
            <div className="mt-4">
                <recharts::BarChart width={1000} height={170} data={counts}>
                    <recharts::CartesianGrid stroke="#1a1a1a" vertical={false} />
                    <recharts::XAxis dataKey="value" stroke="#262626" tickLine={false} />
                    <recharts::YAxis stroke="#262626" tickLine={false} allowDecimals={false} width={32} />
                    <recharts::Tooltip isAnimationActive={false} cursor={false} />
                    <recharts::Bar dataKey="rows" name="Rows" fill="#d98a4a" isAnimationActive={false} />
                </recharts::BarChart>
            </div>
            <p className="mt-2 text-[10px] text-faint">
                How often each value appears in the column above, counted over the whole table.
            </p>
        </div>

        <p className="mt-5 text-[11px] text-muted-foreground">{status.clone()}</p>
    </div>;
}

fn Fact(name: String, value: String) -> Element {
    return <div key={name.clone()} className="rounded-[6px] border border-border bg-card px-4 py-3">
        <div className="text-[9px] tracking-[0.14em] text-faint">{name}</div>
        <div className="mt-1.5 text-[17px] font-semibold tabular-nums">{value}</div>
    </div>;
}

fn Head(at: usize, column: String) -> Element {
    let arrow = if sort_by == at {
        if descending { " ↓" } else { " ↑" }
    } else {
        ""
    };
    return <TableHead key={column.clone()}>
        <button className="text-[10px] tracking-[0.1em] text-muted-foreground hover:text-foreground" onClick={move |_| sort_on(at)}>
            {format!("{column}{arrow}")}
        </button>
    </TableHead>;
}

fn Row(slug: String, at: usize, row: Vec<String>) -> Element {
    return <TableRow key={at.to_string()}>
        {row.iter().enumerate().map(|(cell_at, cell)| Cell(slug.clone(), at, cell_at, cell.clone())).collect::<Vec<_>>()}
        <TableCell className="text-right">
            <button className="text-[10px] text-faint hover:text-foreground" onClick={move |_| drop_row(slug.clone(), at)}>
                remove
            </button>
        </TableCell>
    </TableRow>;
}

/// A cell is editable: a console that can only read a table is a
/// viewer, and the design calls this a console.
fn Cell(slug: String, row_at: usize, cell_at: usize, value: String) -> Element {
    return <TableCell key={format!("{row_at}-{cell_at}")}>
        <input
            className="w-full bg-transparent text-[11px] outline-none"
            value={value}
            onInput={move |event| set_cell(slug.clone(), row_at, cell_at, event.value)}
        />
    </TableCell>;
}

fn Pick(at: usize, column: String) -> Element {
    let here = profile_of == at;
    let classes = format!(
        "rounded-[4px] border px-2 py-[3px] text-[9px] {}",
        if here { "border-[#d98a4a] text-warm" } else { "border-border text-faint hover:text-foreground" }
    );
    return <button key={column.clone()} className={classes} onClick={move |_| profile_of = at}>
        {column}
    </button>;
}

fn Missing(name: String) -> Element {
    return <div className="mt-6 max-w-[520px]">
        <h1 className="text-[18px] font-semibold tracking-tight">No table is called that</h1>
        <p className="mt-2 text-[12px] leading-[1.7] text-muted-foreground">
            {format!("The console holds nothing under “{name}”. Drop a .csv on this window, or go back and pick one that is there.")}
        </p>
        <button className="slab mt-4 rounded-[5px] px-4 py-[8px] text-[11px]" onClick={move |_| rustx::router::navigate("/console".to_string())}>
            Back to the console
        </button>
    </div>;
}

// ---- reading the table ------------------------------------------------

/// The rows the filter leaves, in the order the sort asks for, each
/// one carrying the index it has in the table so that editing a cell
/// of a sorted view edits the right row.
fn visible(table: &Dataset) -> Vec<(usize, Vec<String>)> {
    let wanted = needle.to_lowercase();
    let mut rows: Vec<(usize, Vec<String>)> = table
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            wanted.is_empty() || row.iter().any(|cell| cell.to_lowercase().contains(&wanted))
        })
        .map(|(at, row)| (at, row.clone()))
        .collect();

    let column = sort_by.min(table.columns.len().saturating_sub(1));
    let values: Vec<String> = table
        .rows
        .iter()
        .map(|row| row.get(column).cloned().unwrap_or_default())
        .collect();
    let numeric = all_numbers(&values);

    rows.sort_by(|left, right| {
        let a = left.1.get(column).cloned().unwrap_or_default();
        let b = right.1.get(column).cloned().unwrap_or_default();
        if numeric {
            let a = a.trim().parse::<f64>().unwrap_or(0.0);
            let b = b.trim().parse::<f64>().unwrap_or(0.0);
            a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            a.to_lowercase().cmp(&b.to_lowercase())
        }
    });
    if descending {
        rows.reverse();
    }
    rows
}

/// How often each value appears in a column, biggest first, at most
/// twelve bars so a column of unique keys does not draw a thousand.
fn profile(table: &Dataset, column: usize) -> Vec<Count> {
    let mut counts: Vec<Count> = Vec::new();
    for row in table.rows.iter() {
        let value = row.get(column).cloned().unwrap_or_default();
        match counts.iter_mut().find(|held| held.value == value) {
            Some(held) => held.rows += 1.0,
            None => counts.push(Count { value, rows: 1.0 }),
        }
    }
    counts.sort_by(|left, right| right.rows.partial_cmp(&left.rows).unwrap_or(std::cmp::Ordering::Equal));
    counts.truncate(12);
    counts
}

// ---- what the buttons do ---------------------------------------------

fn sort_on(at: usize) {
    if sort_by == at {
        descending = !descending;
    } else {
        sort_by = at;
        descending = false;
    }
}

fn set_cell(slug: String, row_at: usize, cell_at: usize, value: String) {
    let mut all = tables.clone();
    if let Some(table) = all.iter_mut().find(|table| table.slug == slug) {
        if let Some(row) = table.rows.get_mut(row_at) {
            if let Some(cell) = row.get_mut(cell_at) {
                *cell = value;
            }
        }
    }
    tables = all;
}

fn add_row(slug: String) {
    let mut all = tables.clone();
    if let Some(table) = all.iter_mut().find(|table| table.slug == slug) {
        let blank = vec![String::new(); table.columns.len()];
        table.rows.push(blank);
        status = format!("a row was added to {slug}");
    }
    tables = all;
}

fn drop_row(slug: String, at: usize) {
    let mut all = tables.clone();
    if let Some(table) = all.iter_mut().find(|table| table.slug == slug) {
        if at < table.rows.len() {
            table.rows.remove(at);
        }
    }
    tables = all;
    status = format!("a row was removed from {slug}");
}

fn as_csv(table: &Dataset) -> String {
    let mut text = table.columns.join(",");
    text.push('\n');
    for row in visible(table).iter() {
        text.push_str(&row.1.join(","));
        text.push('\n');
    }
    text
}

fn copy_it(table: Dataset) {
    match rustx::clipboard::set_text(&as_csv(&table)) {
        Ok(()) => status = format!("{} is on the clipboard", table.name),
        Err(error) => status = format!("the clipboard refused: {error}"),
    }
}

fn export_it(table: Dataset) {
    let ask = rustx::dialog::Ask::new("Export this table")
        .filter("Comma separated", ["csv"])
        .named(&format!("{}.csv", table.slug));
    let Ok(path) = rustx::dialog::save(ask) else {
        status = "the export was cancelled".to_string();
        return;
    };
    match std::fs::write(&path, as_csv(&table)) {
        Ok(()) => {
            status = format!("{} written to {}", table.name, path.display());
            let _ = rustx::os::shell::remember_recent(&path);
        }
        Err(error) => status = format!("the export failed: {error}"),
    }
}
