// The frame every screen sits in: the inset navigation bar of the
// design, and the tables the console works on.
//
// The tables live here rather than in a page because they outlive the
// screen — a file dropped on the window while the landing page is up
// must still be there when the console opens. A value a page declares
// is kept by its name for the whole application, so the pages below
// redeclare what they read and get these same cells.

let mut tables: Vec<Dataset> = Vec::new();
let mut status: String = String::new();
let mut imported: u32 = 0;

fn Layout(children: Element) -> Element {
    effect((), || {
        if tables.is_empty() {
            tables = starting_tables();
        }
        // The one moment a dropped file's real path is known: the
        // window is told, not the page.
        rustx::dropped::on_files(move |paths| {
            for path in paths {
                let is_csv = path
                    .extension()
                    .map(|extension| extension.to_ascii_lowercase())
                    == Some("csv".into());
                if is_csv {
                    take_in(&path);
                }
            }
        });
    });

    let here = rustx::router::route();

    return <div className="min-h-screen bg-background text-foreground">
        <div className="px-4 pt-4">
            <header className="flex h-12 items-center gap-7 rounded-[6px] border border-border bg-[#0a0a0a] px-4">
                <button className="flex items-center gap-2" onClick={move |_| rustx::router::navigate("/".to_string())}>
                    <span className="flex h-[22px] w-[22px] items-center justify-center rounded-full border border-[#4a4a4a] text-[11px] font-semibold">
                        R
                    </span>
                </button>
                <nav className="flex items-center gap-6">
                    {NavItem("Architecture".to_string(), "/architecture".to_string(), here.clone())}
                    {NavItem("Platform".to_string(), "/platform".to_string(), here.clone())}
                    {NavItem("Solutions".to_string(), "/solutions".to_string(), here.clone())}
                    {NavItem("Developers".to_string(), "/developers".to_string(), here.clone())}
                    {NavItem("Docs".to_string(), "/docs".to_string(), here.clone())}
                    {NavItem("Company".to_string(), "/company".to_string(), here.clone())}
                    {NavItem("Console".to_string(), "/console".to_string(), here.clone())}
                </nav>
                <button
                    className="slab ml-auto rounded-[5px] px-4 py-[7px] text-[12px]"
                    onClick={move |_| rustx::router::navigate("/contact".to_string())}
                >
                    Contact us
                </button>
            </header>
        </div>

        {children}

        <div className="flex items-center gap-4 border-t border-border px-6 py-3 text-[10px] text-faint">
            <span>R INTELLIGENT DATA</span>
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/docs".to_string())}>Docs</button>
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/console".to_string())}>Console</button>
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/contact".to_string())}>Contact</button>
            <span id="status" className="text-muted-foreground">{status.clone()}</span>
            <span className="ml-auto">
                {format!("{} table(s) · {} imported this session", tables.len(), imported)}
            </span>
        </div>
    </div>;
}

fn NavItem(label: String, route: String, here: String) -> Element {
    let active = here == route || here.starts_with(&format!("{route}/"));
    let classes = format!(
        "text-[12px] {}",
        if active { "text-foreground" } else { "text-muted-foreground hover:text-foreground" }
    );
    let going = route.clone();
    return <button key={label.clone()} className={classes} onClick={move |_| rustx::router::navigate(going.clone())}>
        {label}
    </button>;
}

/// A real file, read off disk, turned into a table.
fn take_in(path: &std::path::Path) {
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
        slug: slug.clone(),
        name: name.clone(),
        about: format!("Read from {}.", path.display()),
        columns,
        rows,
    });
    tables = all;
    imported += 1;
    status = format!("{name} — {count} row(s) read from {}", path.display());
}
