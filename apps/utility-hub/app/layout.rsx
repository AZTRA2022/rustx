// The frame every screen of Reelhouse sits in: the top bar with the
// production's search, the rail of routes down the left, the
// department list where the address calls for one, and Recent
// Activity down the right.
//
// The three lists live here rather than in a page because they outlive
// the screen — importing a crew list on one route and reading it on
// another must be the same list. A value a page declares is kept by
// its name for the whole application, so a page below redeclares the
// handful of values it reads and gets the same cells this layout
// writes.

let mut departments: Vec<Department> = Vec::new();
let mut activity: Vec<Happened> = Vec::new();
let mut note: String = String::new();
let mut dept_filter: String = String::new();
let mut search: String = String::new();
let mut autostart_on: bool = false;

fn Layout(children: Element) -> Element {
    effect((), || {
        if departments.is_empty() {
            departments = starting_departments();
            activity = starting_activity();
        }
        autostart_on = rustx::autostart::is_on("Reelhouse");
    });

    let here = rustx::router::route();
    let with_pane = here.starts_with("/departments") || here.starts_with("/crew");

    return <div className="flex h-screen w-screen flex-col overflow-hidden">
        {TopBar()}
        <div className="flex min-h-0 flex-1">
            {Rail(here.clone())}
            {if with_pane { DepartmentPane(here.clone()) } else { <span></span> }}
            <main className="flex min-w-0 flex-1 flex-col border-r border-border">
                <div className="min-h-0 flex-1 overflow-y-auto">{children}</div>
                {Footer()}
            </main>
            {ActivityPane()}
        </div>
    </div>;
}

fn TopBar() -> Element {
    return <header className="flex h-9 shrink-0 items-center border-b border-border bg-rail">
        <button className="flex w-[180px] items-center gap-2 px-3" onClick={move |_| rustx::router::navigate("/".to_string())}>
            <Clapperboard size={13} color="#7d8894" />
            <span className="text-[12px] font-semibold text-foreground">Reelhouse</span>
        </button>
        <div className="mx-auto flex w-[380px] items-center gap-2 rounded-md border border-border bg-[#11151b] px-2.5 py-[5px]">
            <Search size={12} color="#5a6572" />
            <input
                id="search"
                className="w-full bg-transparent text-[12px] outline-none placeholder:text-faint"
                placeholder="Search for crew, units, or call sheets…"
                value={search.clone()}
                onInput={move |event| search = event.value}
                onKeyDown={move |event| { if event.key == "Enter" { look_for(); } }}
            />
        </div>
        <div className="flex items-center gap-3 px-3">
            <button className="text-faint hover:text-foreground" onClick={move |_| rustx::router::navigate("/help".to_string())}>
                <CircleHelp size={13} color="#5a6572" />
            </button>
            <button onClick={move |_| rustx::router::navigate("/preferences".to_string())}>
                <span style={avatar_paint("Dara Whitlock", 18)}>{initials("Dara Whitlock")}</span>
            </button>
        </div>
    </header>;
}

fn Rail(here: String) -> Element {
    return <aside className="flex w-[180px] shrink-0 flex-col border-r border-border bg-rail px-2 py-3">
        {RailGroup("PRODUCTION".to_string())}
        {RailItem("Dashboard".to_string(), "/".to_string(), here.clone())}
        {RailItem("Call Sheets".to_string(), "/call-sheets".to_string(), here.clone())}
        {RailItem("Footage".to_string(), "/footage".to_string(), here.clone())}

        {RailGroup("CREW".to_string())}
        {RailItem("Departments".to_string(), "/departments".to_string(), here.clone())}
        {RailItem("Roles & Access".to_string(), "/roles".to_string(), here.clone())}
        {RailItem("Assignments".to_string(), "/assignments".to_string(), here.clone())}

        {RailGroup("INSIGHTS".to_string())}
        {RailItem("Reports".to_string(), "/reports".to_string(), here.clone())}
        {RailItem("Budget".to_string(), "/budget".to_string(), here.clone())}

        {RailGroup("SYSTEM".to_string())}
        {RailItem("Preferences".to_string(), "/preferences".to_string(), here.clone())}
        {RailItem("Help Centre".to_string(), "/help".to_string(), here.clone())}

        <div className="mt-auto flex items-center gap-2 rounded-md border border-border bg-[#11151b] px-2 py-2">
            <span style={avatar_paint("Dara Whitlock", 22)}>{initials("Dara Whitlock")}</span>
            <span className="min-w-0">
                <span className="block truncate text-[12px] text-foreground">Dara Whitlock</span>
                <span className="block text-[10px] text-faint">1st AD</span>
            </span>
            <button className="ml-auto text-faint hover:text-foreground" onClick={move |_| rustx::router::navigate("/preferences".to_string())}>
                <EllipsisVertical size={13} color="#5a6572" />
            </button>
        </div>
    </aside>;
}

fn RailGroup(name: String) -> Element {
    return <div key={name.clone()} className="px-2 pb-1 pt-4 text-[9px] tracking-[0.14em] text-faint">{name}</div>;
}

fn RailItem(label: String, route: String, here: String) -> Element {
    // Departments owns the crew pages too: a person opened from a
    // roster is still the crew section of the rail.
    let active = if route == "/" {
        here == "/"
    } else if route == "/departments" {
        here.starts_with("/departments") || here.starts_with("/crew")
    } else {
        here.starts_with(&route)
    };
    let classes = format!(
        "flex w-full items-center gap-2.5 rounded-md px-2 py-[7px] text-[12px] {}",
        if active { "bg-bluesoft text-[#6ea0ff]" } else { "text-muted hover:text-foreground" }
    );
    let going = route.clone();
    return <button key={label.clone()} className={classes} onClick={move |_| rustx::router::navigate(going.clone())}>
        {RailIcon(label.clone(), active)}
        <span>{label}</span>
    </button>;
}

fn RailIcon(label: String, active: bool) -> Element {
    let tint = if active { "#6ea0ff" } else { "#7d8894" };
    if label == "Dashboard" {
        return <LayoutGrid size={13} color={tint.to_string()} />;
    }
    if label == "Call Sheets" {
        return <Folder size={13} color={tint.to_string()} />;
    }
    if label == "Footage" {
        return <Film size={13} color={tint.to_string()} />;
    }
    if label == "Departments" {
        return <Network size={13} color={tint.to_string()} />;
    }
    if label == "Roles & Access" {
        return <KeyRound size={13} color={tint.to_string()} />;
    }
    if label == "Assignments" {
        return <ClipboardList size={13} color={tint.to_string()} />;
    }
    if label == "Reports" {
        return <ChartColumn size={13} color={tint.to_string()} />;
    }
    if label == "Budget" {
        return <Wallet size={13} color={tint.to_string()} />;
    }
    if label == "Preferences" {
        return <SlidersHorizontal size={13} color={tint.to_string()} />;
    }
    return <CircleHelp size={13} color={tint.to_string()} />;
}

/// The second column of the mock. It belongs to the layout and not to
/// the department page because it stays put while the three tabs
/// change under it — and because a person's own page, which is not
/// under `/departments`, still shows the unit it came from.
fn DepartmentPane(here: String) -> Element {
    let needle = dept_filter.to_lowercase();
    return <aside className="flex w-[200px] shrink-0 flex-col border-r border-border bg-pane px-2.5 py-3">
        <button className="px-1 text-left text-[12px] font-semibold text-foreground" onClick={move |_| rustx::router::navigate("/departments".to_string())}>
            All Departments
        </button>
        <div className="mt-2.5 flex items-center gap-2 rounded-md border border-border bg-[#11151b] px-2 py-[5px]">
            <Search size={11} color="#5a6572" />
            <input
                id="dept-filter"
                className="w-full bg-transparent text-[11px] outline-none placeholder:text-faint"
                placeholder="Filter departments"
                value={dept_filter.clone()}
                onInput={move |event| dept_filter = event.value}
            />
        </div>
        <div id="departments" className="mt-2 flex flex-col gap-0.5 overflow-y-auto">
            {departments
                .iter()
                .filter(|department| needle.is_empty() || department.name.to_lowercase().contains(&needle))
                .map(|department| DepartmentRow(department.clone(), here.clone()))
                .collect::<Vec<_>>()}
        </div>
    </aside>;
}

fn DepartmentRow(department: Department, here: String) -> Element {
    let root = format!("/departments/{}", department.slug);
    let active = here == root || here.starts_with(&format!("{root}/"));
    let classes = format!(
        "rounded-md px-2.5 py-2 text-left {}",
        if active { "bg-blue text-white" } else { "text-foreground hover:bg-[#141922]" }
    );
    let subtle = if active { "text-[10px] text-[#cfe0ff]" } else { "text-[10px] text-faint" };
    let going = root.clone();
    return <button key={department.slug.clone()} className={classes} onClick={move |_| rustx::router::navigate(going.clone())}>
        <span className="block text-[12px]">{department.name.clone()}</span>
        <span className={subtle}>{format!("{} members", department.crew.len())}</span>
    </button>;
}

fn Footer() -> Element {
    return <div className="flex shrink-0 items-center border-t border-border px-5 py-2 text-[11px] text-faint">
        <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/support".to_string())}>Support</button>
        <button className="ml-3 hover:text-foreground" onClick={move |_| rustx::router::navigate("/feedback".to_string())}>Feedback</button>
        <span id="note" className="mx-4 truncate text-muted">{note.clone()}</span>
        <span className="ml-auto shrink-0">Reelhouse v2.3.1</span>
    </div>;
}

fn ActivityPane() -> Element {
    return <aside className="flex w-[250px] shrink-0 flex-col overflow-y-auto bg-pane px-4 py-4">
        <div className="text-[12px] font-semibold text-foreground">Recent Activity</div>
        <div className="mt-0.5 text-[10px] text-faint">Latest updates from the production</div>

        <div className="mt-4 text-[9px] tracking-[0.14em] text-faint">TODAY</div>
        <div id="today" className="mt-2 flex flex-col gap-3.5">
            {activity.iter().enumerate().filter(|(_, item)| !item.week).map(|(index, item)| ActivityRow(index, item.clone())).collect::<Vec<_>>()}
        </div>

        <div className="mt-5 text-[9px] tracking-[0.14em] text-faint">THIS WEEK</div>
        <div className="mt-2 flex flex-col gap-3.5">
            {activity.iter().enumerate().filter(|(_, item)| item.week).map(|(index, item)| ActivityRow(index, item.clone())).collect::<Vec<_>>()}
        </div>
    </aside>;
}

fn ActivityRow(index: usize, item: Happened) -> Element {
    return <div key={format!("{}-{}", index, item.who)} className="flex items-start gap-2.5">
        <span style={avatar_paint(&item.who, 22)}>{initials(&item.who)}</span>
        <span className="min-w-0 flex-1">
            <span className="flex items-baseline">
                <span className="truncate text-[12px] text-foreground">{item.who.clone()}</span>
                <span className="ml-auto shrink-0 pl-2 text-[10px] text-faint">{item.when_said.clone()}</span>
            </span>
            <span className="mt-0.5 block text-[11px] leading-snug text-muted">{item.what.clone()}</span>
        </span>
    </div>;
}

/// Enter in the top bar goes to the results page, with the term in the
/// address so the result of a search can be gone back to.
fn look_for() {
    let term = search.trim().to_string();
    if term.is_empty() {
        note = "type something to search for".to_string();
        return;
    }
    rustx::router::navigate(format!("/search?q={}", term.replace(' ', "+")));
}
