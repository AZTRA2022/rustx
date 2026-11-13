// R Intelligent Data — the table browser.
//
// Tables live in memory as plain Rust — `Vec<Table>` — and the only
// two ways data gets in are real ones: dropping a `.csv` file on the
// window (its real path, read from disk, via `rustx::dropped`) or the
// system's own "open file" dialog (`rustx::dialog`). The grid is
// shadcn/ui's `Table` on Tailwind v4, and the row-count chart is
// `recharts`, off npm — not hand-rolled look-alikes.

#[derive(rustx::ToValue, Clone)]
struct Bar {
    table: String,
    rows: f64,
}

#[derive(Clone)]
struct Table_ {
    name: String,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

let mut tables: Vec<Table_> = vec![
    Table_ {
        name: "users".to_string(),
        columns: vec!["id".to_string(), "name".to_string(), "plan".to_string()],
        rows: vec![
            vec!["1".to_string(), "Ada".to_string(), "pro".to_string()],
            vec!["2".to_string(), "Grace".to_string(), "free".to_string()],
            vec!["3".to_string(), "Linus".to_string(), "pro".to_string()],
        ],
    },
    Table_ {
        name: "orders".to_string(),
        columns: vec!["id".to_string(), "user_id".to_string(), "total".to_string()],
        rows: vec![
            vec!["101".to_string(), "1".to_string(), "42.00".to_string()],
            vec!["102".to_string(), "3".to_string(), "19.90".to_string()],
        ],
    },
];
let mut selected: usize = 0;
let mut status = String::new();

fn Page() -> Element {
    effect((), || {
        rustx::dropped::on_files(move |paths| {
            for path in paths {
                if path.extension().map(|extension| extension.to_ascii_lowercase()) == Some("csv".into()) {
                    import_csv(&path);
                }
            }
        });
    });

    return <div className="mx-auto max-w-6xl px-6 py-8">
        <div className="grid grid-cols-1 gap-5 lg:grid-cols-[260px_1fr]">
            {Sidebar()}
            <div className="flex flex-col gap-5">
                {Overview()}
                {Main()}
            </div>
        </div>
    </div>;
}

fn Sidebar() -> Element {
    return <Card className="border-border bg-card">
        <CardHeader>
            <CardTitle className="text-xs uppercase tracking-wider text-muted-foreground">{tables.len()} table(s)</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
            <ul id="tables" className="flex flex-col gap-1">
                {tables.iter().enumerate().map(|(index, table)| TableTab(index, table.name.clone(), table.rows.len())).collect::<Vec<_>>()}
            </ul>

            <Button variant="outline" id="import" className="mt-2" onClick={move |_| {
                let Ok(paths) = rustx::dialog::open(
                    rustx::dialog::Ask::new("Import a CSV table").filter("CSV", ["csv"]),
                ) else { return; };
                if let Some(path) = paths.first() {
                    import_csv(path);
                }
            }}>
                import CSV…
            </Button>
            <p className="text-[11px] text-muted-foreground">or drop a .csv file on this window</p>
            <p id="status" className="text-xs text-emerald-400">{status.clone()}</p>
        </CardContent>
    </Card>;
}

fn TableTab(index: usize, name: String, rows: usize) -> Element {
    let active = selected == index;
    return <li key={index.to_string()}
        onClick={move |_| selected = index}
        className={format!("flex cursor-pointer items-center justify-between rounded-md px-2.5 py-1.5 {}", if active { "bg-accent" } else { "" })}>
        <span>{name.clone()}</span>
        <span className="text-muted-foreground">{rows}</span>
    </li>;
}

fn Overview() -> Element {
    let data: Vec<Bar> = tables.iter().map(|table| Bar { table: table.name.clone(), rows: table.rows.len() as f64 }).collect();

    return <Card className="border-border bg-card">
        <CardHeader>
            <CardTitle className="text-xs uppercase tracking-wider text-muted-foreground">rows per table</CardTitle>
        </CardHeader>
        <CardContent>
            <recharts::BarChart width={660} height={160} data={data}>
                <recharts::XAxis dataKey="table" stroke="#52525b" />
                <recharts::YAxis stroke="#52525b" allowDecimals={false} />
                <recharts::Bar dataKey="rows" fill="#6366f1" />
            </recharts::BarChart>
        </CardContent>
    </Card>;
}

fn Main() -> Element {
    if tables.is_empty() {
        return <Card className="border-border bg-card"><CardContent>no tables yet.</CardContent></Card>;
    }
    let index = selected.min(tables.len() - 1);
    let table = tables[index].clone();
    let route_name = table.name.clone();

    return <Card className="border-border bg-card">
        <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle className="text-base">{table.name.clone()}</CardTitle>
            <div className="flex gap-2">
                <Button variant="ghost" className="h-8 px-3 text-xs" onClick={move |_| rustx::router::navigate(format!("/table/{}", route_name))}>
                    open →
                </Button>
                <Button variant="destructive" className="h-8 px-3 text-xs" onClick={move |_| {
                    tables.remove(index);
                    if selected > 0 { selected -= 1; }
                }}>
                    drop table
                </Button>
            </div>
        </CardHeader>
        <CardContent>
            <Table>
                <TableHeader>
                    <TableRow>
                        {table.columns.iter().map(|column| <TableHead key={column.clone()}>{column.clone()}</TableHead>).collect::<Vec<_>>()}
                        <TableHead></TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {table.rows.iter().enumerate().map(|(row_index, row)| DataRow(index, row_index, row.clone())).collect::<Vec<_>>()}
                </TableBody>
            </Table>
        </CardContent>
    </Card>;
}

fn DataRow(table_index: usize, row_index: usize, row: Vec<String>) -> Element {
    return <TableRow key={row_index.to_string()}>
        {row.iter().map(|cell| <TableCell>{cell.clone()}</TableCell>).collect::<Vec<_>>()}
        <TableCell className="text-right">
            <Button variant="ghost" className="h-7 px-2 text-xs" onClick={move |_| {
                if let Some(table) = tables.get_mut(table_index) {
                    if row_index < table.rows.len() {
                        table.rows.remove(row_index);
                    }
                }
            }}>remove</Button>
        </TableCell>
    </TableRow>;
}

/// Reads a real file from a real path and turns its header row into
/// columns, the rest into rows. No network, no driver: just `std::fs`.
fn import_csv(path: &std::path::Path) {
    let Ok(content) = std::fs::read_to_string(path) else {
        status = format!("could not read {}", path.display());
        return;
    };
    let mut lines = content.lines();
    let Some(header) = lines.next() else {
        status = "empty file".to_string();
        return;
    };
    let columns: Vec<String> = header.split(',').map(|cell| cell.trim().to_string()).collect();
    let rows: Vec<Vec<String>> = lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split(',').map(|cell| cell.trim().to_string()).collect())
        .collect();

    let name = path.file_stem().map(|stem| stem.to_string_lossy().to_string()).unwrap_or_else(|| "imported".to_string());
    let row_count = rows.len();
    tables.push(Table_ { name: name.clone(), columns, rows });
    selected = tables.len() - 1;
    status = format!("imported {} ({} rows)", name, row_count);
}
