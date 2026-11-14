// DEVELOPERS — what someone who writes code gets, shown in the one
// currency a developer trusts: the actual source of this application.

let mut tables: Vec<Dataset> = Vec::new();
let mut status: String = String::new();

let mut tab: usize = 0;

fn Page() -> Element {
    let all = snippets();
    let here = tab.min(all.len() - 1);
    let one = all[here].clone();
    let to_copy = one.2.clone();

    return <div key="developers" id="screen-developers" className="mx-auto max-w-[1000px] px-6 py-10">
        <div className="text-[10px] tracking-[0.16em] text-faint">DEVELOPERS</div>
        <h1 className="mt-2 text-[22px] font-semibold tracking-tight">The parts you would have to write</h1>
        <p className="mt-2.5 max-w-[620px] text-[12px] leading-[1.8] text-muted-foreground">
            Every snippet below is taken from this application, not from a brochure. The console
            you can open from the bar above is the code on this page, running.
        </p>

        <div className="mt-7 flex gap-1.5">
            {all.iter().enumerate().map(|(at, item)| Tab(at, item.0.clone(), at == here)).collect::<Vec<_>>()}
        </div>

        <div className="mt-4 rounded-[6px] border border-border bg-card">
            <div className="flex items-center border-b border-border px-5 py-3">
                <span className="text-[11px] text-muted-foreground">{one.1.clone()}</span>
                <button className="slab ml-auto rounded-[5px] px-3 py-[5px] text-[10px]" onClick={move |_| copy_it(to_copy.clone())}>
                    COPY
                </button>
            </div>
            <pre className="overflow-auto px-5 py-4 text-[11px] leading-[1.8] text-foreground">{one.2.clone()}</pre>
        </div>

        <div className="mt-6 grid grid-cols-3 gap-4">
            {Fact("TABLES IN THIS WINDOW".to_string(), tables.len().to_string())}
            {Fact("ROWS".to_string(), tables.iter().map(|table| table.rows.len()).sum::<usize>().to_string())}
            {Fact("ROUTES IN THIS APPLICATION".to_string(), "11".to_string())}
        </div>

        <div className="mt-6 flex gap-2">
            <button className="slab rounded-[5px] px-4 py-[8px] text-[11px]" onClick={move |_| rustx::router::navigate("/docs".to_string())}>
                READ THE DOCS
            </button>
            <button className="glossy rounded-[5px] px-4 py-[8px] text-[11px]" onClick={move |_| rustx::router::navigate("/console".to_string())}>
                OPEN THE CONSOLE
            </button>
        </div>

        <p className="mt-4 text-[11px] text-muted-foreground">{status.clone()}</p>
    </div>;
}

fn Tab(at: usize, name: String, here: bool) -> Element {
    let classes = format!(
        "rounded-[5px] border px-3.5 py-[7px] text-[11px] {}",
        if here { "border-[#d98a4a] text-warm" } else { "border-border text-muted-foreground hover:text-foreground" }
    );
    return <button key={name.clone()} className={classes} onClick={move |_| tab = at}>
        {name}
    </button>;
}

fn Fact(name: String, value: String) -> Element {
    return <div key={name.clone()} className="rounded-[6px] border border-border bg-card px-4 py-3">
        <div className="text-[9px] tracking-[0.14em] text-faint">{name}</div>
        <div className="mt-1.5 text-[17px] font-semibold tabular-nums">{value}</div>
    </div>;
}

fn copy_it(text: String) {
    match rustx::clipboard::set_text(&text) {
        Ok(()) => status = "the snippet is on the clipboard".to_string(),
        Err(error) => status = format!("the clipboard refused: {error}"),
    }
}

/// Name, where it is from, and the source itself.
fn snippets() -> Vec<(String, String, String)> {
    vec![
        (
            "A route".to_string(),
            "app/console/[name]/page.rsx — a page is a file, and a folder is a segment".to_string(),
            "// app/console/[name]/page.rsx  ->  /console/<name>\nlet mut tables: Vec<Dataset> = Vec::new();\n\nfn Page(name: String) -> Element {\n    let found = tables.iter().find(|table| table.slug == name).cloned();\n\n    return <div className=\"px-6 py-9\">\n        {match found {\n            Some(table) => Body(table),\n            None => Missing(name.clone()),\n        }}\n    </div>;\n}".to_string(),
        ),
        (
            "Shared state".to_string(),
            "app/layout.rsx — a value is kept by its name for the whole application".to_string(),
            "// Declared in the layout, redeclared by any page that reads it.\n// Both names refer to the same cell; nothing is passed down.\nlet mut tables: Vec<Dataset> = Vec::new();\nlet mut status: String = String::new();\n\nfn Layout(children: Element) -> Element {\n    effect((), || {\n        if tables.is_empty() {\n            tables = starting_tables();\n        }\n    });\n    return <div>{children}</div>;\n}".to_string(),
        ),
        (
            "A dropped file".to_string(),
            "app/layout.rsx — the one place a dropped file's real path is known".to_string(),
            "rustx::dropped::on_files(move |paths| {\n    for path in paths {\n        let is_csv = path\n            .extension()\n            .map(|extension| extension.to_ascii_lowercase())\n            == Some(\"csv\".into());\n        if is_csv {\n            take_in(&path);\n        }\n    }\n});".to_string(),
        ),
        (
            "A chart".to_string(),
            "app/console/page.rsx — recharts, off npm, fed a Rust value".to_string(),
            "#[derive(rustx::ToValue, Clone)]\nstruct Bar {\n    table: String,\n    rows: f64,\n    columns: f64,\n}\n\n<recharts::BarChart width={1000} height={170} data={bars}>\n    <recharts::XAxis dataKey=\"table\" stroke=\"#262626\" />\n    <recharts::YAxis stroke=\"#262626\" allowDecimals={false} />\n    <recharts::Bar dataKey=\"rows\" fill=\"#d98a4a\" />\n</recharts::BarChart>".to_string(),
        ),
        (
            "A real file out".to_string(),
            "app/console/[name]/page.rsx — the system's own save dialog".to_string(),
            "let ask = rustx::dialog::Ask::new(\"Export this table\")\n    .filter(\"Comma separated\", [\"csv\"])\n    .named(&format!(\"{}.csv\", table.slug));\nlet Ok(path) = rustx::dialog::save(ask) else { return };\n\nstd::fs::write(&path, as_csv(&table))?;\nrustx::os::shell::remember_recent(&path)?;".to_string(),
        ),
    ]
}
