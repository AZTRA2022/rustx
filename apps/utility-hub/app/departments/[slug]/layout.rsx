// What the three tabs of a department have in common: the breadcrumb,
// the Import / Export / New Crew buttons, and the tab strip — which is
// three routes and not three states, so the address says which sheet
// is open.

let mut departments: Vec<Department> = Vec::new();
let mut activity: Vec<Happened> = Vec::new();
let mut note: String = String::new();

fn Layout(children: Element, slug: String) -> Element {
    let department = departments.iter().find(|held| held.slug == slug).cloned();
    let name = department.as_ref().map(|held| held.name.clone()).unwrap_or_else(|| "Unknown".to_string());
    let here = rustx::router::route();
    let sheet = if here.ends_with("/call-times") {
        "Call Times"
    } else if here.ends_with("/kit") {
        "Kit"
    } else {
        "Crew"
    };

    return <div className="flex min-h-full flex-col px-5 py-4">
        <div className="flex items-start">
            <div>
                <h1 className="flex items-center gap-1.5 text-[13px] font-semibold text-foreground">
                    <button className="text-muted hover:text-foreground" onClick={move |_| rustx::router::navigate("/departments".to_string())}>
                        Departments
                    </button>
                    <span className="text-faint">→</span>
                    <span>{name.clone()}</span>
                    <span className="text-faint">→</span>
                    <span>{sheet}</span>
                </h1>
                <p className="mt-0.5 text-[11px] text-muted">Manage your unit and track call times in real time.</p>
            </div>
            <div className="ml-auto flex items-center gap-2">
                <button
                    className="flex items-center gap-1.5 rounded-md border border-border bg-[#11151b] px-2.5 py-[6px] text-[11px] text-foreground hover:border-faint"
                    onClick={move |_| import_crew(slug.clone())}
                >
                    <Upload size={11} color="#c9d1d9" />
                    Import
                </button>
                <button
                    className="flex items-center gap-1.5 rounded-md border border-border bg-[#11151b] px-2.5 py-[6px] text-[11px] text-foreground hover:border-faint"
                    onClick={move |_| export_crew(slug.clone())}
                >
                    <Download size={11} color="#c9d1d9" />
                    Export
                </button>
                <button
                    className="flex items-center gap-1.5 rounded-md bg-blue px-2.5 py-[6px] text-[11px] text-white hover:brightness-110"
                    onClick={move |_| hire(slug.clone())}
                >
                    <Plus size={11} color="#ffffff" />
                    New Crew
                </button>
            </div>
        </div>

        <div className="mt-3.5 flex w-fit rounded-md border border-border bg-[#11151b] p-[3px]">
            {Tab("Crew".to_string(), format!("/departments/{slug}"), here.clone())}
            {Tab("Call Times".to_string(), format!("/departments/{slug}/call-times"), here.clone())}
            {Tab("Kit".to_string(), format!("/departments/{slug}/kit"), here.clone())}
        </div>

        <div className="mt-3 flex-1">{children}</div>
    </div>;
}

fn Tab(label: String, route: String, here: String) -> Element {
    let active = here == route;
    let classes = format!(
        "rounded-[3px] px-3 py-[5px] text-[11px] {}",
        if active { "bg-[#1c222b] text-foreground" } else { "text-muted hover:text-foreground" }
    );
    let going = route.clone();
    return <button key={label.clone()} className={classes} onClick={move |_| rustx::router::navigate(going.clone())}>
        {label}
    </button>;
}

// ---- what the buttons do ---------------------------------------------

/// A real file, read off disk, whose rows join the department that is
/// open.
fn import_crew(slug: String) {
    let ask = rustx::dialog::Ask::new("Import a crew list").filter("Comma separated", ["csv"]);
    let Ok(paths) = rustx::dialog::open(ask) else {
        note = "the import was cancelled".to_string();
        return;
    };
    let Some(path) = paths.first() else {
        note = "the import was cancelled".to_string();
        return;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        note = format!("{} could not be read", path.display());
        return;
    };

    let mut joined = 0usize;
    let mut all = departments.clone();
    if let Some(department) = all.iter_mut().find(|held| held.slug == slug) {
        for line in text.lines().skip(1) {
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() < 2 || fields[0].trim().is_empty() {
                continue;
            }
            let email = fields[1].trim().to_string();
            department.crew.push(Person {
                id: email.split('@').next().unwrap_or("imported").to_string(),
                name: fields[0].trim().to_string(),
                role: fields.get(2).map(|role| role.trim().to_string()).unwrap_or_else(|| "Trainee".to_string()),
                email,
            });
            joined += 1;
        }
    }
    departments = all;
    note = format!("{joined} imported from {}", path.display());
    remember("imported a crew list".to_string(), "Dara Whitlock".to_string());
}

/// A real CSV, through the system's own save dialog, announced with a
/// real notification.
fn export_crew(slug: String) {
    let Some(department) = departments.iter().find(|held| held.slug == slug).cloned() else {
        return;
    };
    let ask = rustx::dialog::Ask::new("Export this department")
        .filter("Comma separated", ["csv"])
        .named(&format!("{}.csv", department.slug));
    let Ok(path) = rustx::dialog::save(ask) else {
        note = "the export was cancelled".to_string();
        return;
    };

    let mut text = String::from("name,email,role\n");
    for person in department.crew.iter() {
        text.push_str(&format!("{},{},{}\n", person.name, person.email, person.role));
    }

    match std::fs::write(&path, text) {
        Ok(()) => {
            note = format!("{} people written to {}", department.crew.len(), path.display());
            let _ = rustx::notify::show(rustx::notify::What::new(
                "Reelhouse",
                format!("{} exported — {} people.", department.name, department.crew.len()),
            ));
            let _ = rustx::os::shell::remember_recent(&path);
        }
        Err(error) => note = format!("the export failed: {error}"),
    }
}

fn hire(slug: String) {
    let mut all = departments.clone();
    let Some(department) = all.iter_mut().find(|held| held.slug == slug) else {
        return;
    };
    let number = department.crew.len() + 1;
    let name = format!("New Crew {number}");
    department.crew.push(Person {
        id: format!("{slug}-{number}"),
        email: format!("{slug}{number}@reelhouse.co"),
        role: "Trainee".to_string(),
        name: name.clone(),
    });
    departments = all;
    note = format!("{name} added");
    remember("added a crew member".to_string(), "Dara Whitlock".to_string());
}

fn remember(what: String, who: String) {
    let mut all = activity.clone();
    all.insert(0, Happened { who, what, when_said: "just now".to_string(), week: false });
    all.truncate(14);
    activity = all;
}
