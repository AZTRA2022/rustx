// DASHBOARD — the production at a glance, and the first thing the
// rail points at.
//
// Every number here is counted off the lists the root layout holds,
// so importing a crew list changes this page too. Every card is a
// link into the screen it summarises.

let mut departments: Vec<Department> = Vec::new();
let mut activity: Vec<Happened> = Vec::new();

fn Page() -> Element {
    let people: usize = departments.iter().map(|department| department.crew.len()).sum();
    let leads: usize = departments
        .iter()
        .map(|department| department.crew.iter().filter(|person| person.role == "Lead").count())
        .sum();
    let sheets = call_sheets();
    let next = sheets.iter().find(|sheet| !sheet.locked).cloned();
    let clips = footage();
    let graded = clips.iter().filter(|clip| clip.graded).count();
    let budget = budget_lines();
    let planned: f64 = budget.iter().map(|line| line.planned).sum();
    let spent: f64 = budget.iter().map(|line| line.spent).sum();

    return <div key="dashboard" className="px-5 py-4">
        <h1 className="text-[13px] font-semibold text-foreground">Dashboard</h1>
        <p className="mt-0.5 text-[11px] text-muted">
            {match next.clone() {
                Some(sheet) => format!("Next up: {} — {} call at {}.", sheet.day, sheet.unit.to_lowercase(), sheet.call),
                None => "Every sheet on the schedule is locked.".to_string(),
            }}
        </p>

        <div className="mt-4 grid grid-cols-4 gap-2.5">
            {Tile("CREW".to_string(), people.to_string(), format!("{} departments · {leads} leads", departments.len()), "/departments".to_string())}
            {Tile("SHOOT DAYS".to_string(), sheets.len().to_string(), format!("{} locked", sheets.iter().filter(|sheet| sheet.locked).count()), "/call-sheets".to_string())}
            {Tile("CLIPS".to_string(), clips.len().to_string(), format!("{graded} graded"), "/footage".to_string())}
            {Tile("BUDGET".to_string(), format!("{:.0}%", if planned == 0.0 { 0.0 } else { spent * 100.0 / planned }), format!("{} of {}", money(spent), money(planned)), "/budget".to_string())}
        </div>

        <div className="mt-5 grid grid-cols-[1.1fr_1fr] gap-3">
            <div className="rounded-md border border-border bg-[#0f131a] p-4">
                <div className="flex items-center">
                    <span className="text-[9px] tracking-[0.14em] text-faint">THE UNIT</span>
                    <button className="ml-auto text-[10px] text-muted hover:text-foreground" onClick={move |_| rustx::router::navigate("/departments".to_string())}>
                        All departments
                    </button>
                </div>
                <div className="mt-3 flex flex-col gap-2">
                    {departments.iter().map(|department| Bar(department.clone(), people)).collect::<Vec<_>>()}
                </div>
            </div>

            <div className="rounded-md border border-border bg-[#0f131a] p-4">
                <div className="flex items-center">
                    <span className="text-[9px] tracking-[0.14em] text-faint">THE WEEK AHEAD</span>
                    <button className="ml-auto text-[10px] text-muted hover:text-foreground" onClick={move |_| rustx::router::navigate("/call-sheets".to_string())}>
                        All call sheets
                    </button>
                </div>
                <div className="mt-3 flex flex-col gap-1.5">
                    {sheets.iter().map(|sheet| Day(sheet.clone())).collect::<Vec<_>>()}
                </div>
            </div>
        </div>

        <div className="mt-5 rounded-md border border-border bg-[#0f131a] p-4">
            <div className="text-[9px] tracking-[0.14em] text-faint">WHAT HAPPENED TODAY</div>
            <div className="mt-3 flex flex-col gap-2">
                {activity.iter().filter(|item| !item.week).map(|item| Line(item.clone())).collect::<Vec<_>>()}
                {if activity.iter().all(|item| item.week) {
                    <span className="text-[11px] text-faint">Nothing yet today.</span>
                } else {
                    <span></span>
                }}
            </div>
        </div>
    </div>;
}

fn Tile(name: String, value: String, under: String, route: String) -> Element {
    let going = route.clone();
    return <button
        key={name.clone()}
        className="rounded-md border border-border bg-[#0f131a] px-3.5 py-3 text-left hover:border-faint"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="block text-[9px] tracking-[0.12em] text-faint">{name}</span>
        <span className="mt-1.5 block text-[20px] font-semibold tabular-nums text-foreground">{value}</span>
        <span className="mt-0.5 block truncate text-[10px] text-muted">{under}</span>
    </button>;
}

fn Bar(department: Department, total: usize) -> Element {
    let share = if total == 0 { 0.0 } else { department.crew.len() as f64 * 100.0 / total as f64 };
    let paint = format!("height:6px;border-radius:3px;background-color:#3b6fd4;width:{share}%");
    let going = format!("/departments/{}", department.slug);
    return <button key={department.slug.clone()} className="text-left" onClick={move |_| rustx::router::navigate(going.clone())}>
        <span className="flex items-baseline text-[11px]">
            <span className="text-foreground">{department.name.clone()}</span>
            <span className="ml-auto text-muted">{format!("{}", department.crew.len())}</span>
        </span>
        <span className="mt-1 block h-[6px] rounded-[3px] bg-[#151b24]">
            <span className="block" style={paint}></span>
        </span>
    </button>;
}

fn Day(sheet: CallSheet) -> Element {
    let going = format!("/call-sheets/{}", sheet.slug);
    return <button
        key={sheet.slug.clone()}
        className="flex items-center gap-2 rounded-md border border-border bg-[#11151b] px-2.5 py-[7px] text-left hover:border-faint"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="text-[11px] text-foreground">{sheet.day.clone()}</span>
        <span className="text-[10px] text-faint">{sheet.date.clone()}</span>
        <span className="ml-auto text-[10px] tabular-nums text-muted">{format!("call {}", sheet.call)}</span>
        {if sheet.locked {
            <span className="rounded-[3px] bg-[#1a1f27] px-1.5 py-[1px] text-[9px] text-[#9aa5b1]">LOCKED</span>
        } else {
            <span className="rounded-[3px] bg-[#11291d] px-1.5 py-[1px] text-[9px] text-[#4ec98a]">OPEN</span>
        }}
    </button>;
}

fn Line(item: Happened) -> Element {
    return <div key={format!("{}-{}", item.who, item.what)} className="flex items-center gap-2.5">
        <span style={avatar_paint(&item.who, 20)}>{initials(&item.who)}</span>
        <span className="text-[11px] text-foreground">{item.who.clone()}</span>
        <span className="text-[11px] text-muted">{item.what.clone()}</span>
        <span className="ml-auto text-[10px] text-faint">{item.when_said.clone()}</span>
    </div>;
}

/// A sum in the production's own currency, short enough for a tile.
fn money(amount: f64) -> String {
    if amount >= 1_000_000.0 {
        return format!("{:.2}M", amount / 1_000_000.0);
    }
    format!("{:.0}k", amount / 1_000.0)
}
