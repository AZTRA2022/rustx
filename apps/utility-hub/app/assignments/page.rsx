// ASSIGNMENTS — who is on which day.
//
// The grid is built from the live department list against the
// schedule, so it changes when a crew list is imported. A cell is a
// switch: taking someone off a day, or putting them on one, is real
// state the counts at the foot read back.

let mut departments: Vec<Department> = Vec::new();
let mut activity: Vec<Happened> = Vec::new();
let mut note: String = String::new();
let mut off: Vec<String> = Vec::new();
let mut unit: String = "camera".to_string();

fn Page() -> Element {
    let sheets = call_sheets();
    let crew = departments
        .iter()
        .find(|department| department.slug == unit)
        .map(|department| department.crew.clone())
        .unwrap_or_default();
    let away = off.clone();
    let booked: usize = crew
        .iter()
        .map(|person| sheets.iter().filter(|sheet| !away.contains(&tag(&person.id, &sheet.slug))).count())
        .sum();

    return <div key="assignments" className="px-5 py-4">
        <div className="flex items-start">
            <div>
                <h1 className="text-[13px] font-semibold text-foreground">Assignments</h1>
                <p className="mt-0.5 text-[11px] text-muted">
                    {format!("{booked} person-days booked across {} shooting days.", sheets.len())}
                </p>
            </div>
            <div className="ml-auto flex flex-wrap items-center justify-end gap-1.5">
                {departments.iter().map(|department| Pick(department.slug.clone(), department.name.clone())).collect::<Vec<_>>()}
            </div>
        </div>

        <div id="grid" className="mt-4 overflow-hidden rounded-md border border-border bg-[#0f131a]">
            <div className="flex border-b border-border px-3.5 py-2 text-[10px] tracking-[0.1em] text-faint">
                <span className="w-[190px]">PERSON</span>
                {sheets.iter().map(|sheet| Head(sheet.clone())).collect::<Vec<_>>()}
                <span className="ml-auto w-[48px] text-right">DAYS</span>
            </div>
            {crew.iter().map(|person| Row(person.clone(), sheets.clone())).collect::<Vec<_>>()}
            {if crew.is_empty() {
                <div className="px-3.5 py-6 text-[11px] text-faint">This department has nobody in it yet.</div>
            } else {
                <span></span>
            }}
        </div>
    </div>;
}

fn Pick(slug: String, name: String) -> Element {
    let active = unit == slug;
    let classes = format!(
        "rounded-md px-2.5 py-[5px] text-[11px] {}",
        if active { "bg-blue text-white" } else { "border border-border bg-[#11151b] text-muted hover:text-foreground" }
    );
    let chosen = slug.clone();
    return <button key={slug.clone()} className={classes} onClick={move |_| unit = chosen.clone()}>
        {name}
    </button>;
}

fn Head(sheet: CallSheet) -> Element {
    let going = format!("/call-sheets/{}", sheet.slug);
    return <button key={sheet.slug.clone()} className="w-[78px] text-left hover:text-foreground" onClick={move |_| rustx::router::navigate(going.clone())}>
        {sheet.day.replace("Day ", "D")}
    </button>;
}

fn Row(person: Person, sheets: Vec<CallSheet>) -> Element {
    let away = off.clone();
    let days = sheets.iter().filter(|sheet| !away.contains(&tag(&person.id, &sheet.slug))).count();
    let going = format!("/crew/{}", person.id);

    return <div key={person.id.clone()} className="flex items-center border-b border-border px-3.5 py-[7px] text-[11px] last:border-b-0">
        <button className="flex w-[190px] items-center gap-2 text-left hover:text-white" onClick={move |_| rustx::router::navigate(going.clone())}>
            <span style={avatar_paint(&person.name, 18)}>{initials(&person.name)}</span>
            <span className="truncate text-foreground">{person.name.clone()}</span>
        </button>
        {sheets.iter().map(|sheet| Cell(person.id.clone(), person.name.clone(), sheet.clone())).collect::<Vec<_>>()}
        <span className="ml-auto w-[48px] text-right tabular-nums text-muted">{days.to_string()}</span>
    </div>;
}

fn Cell(id: String, name: String, sheet: CallSheet) -> Element {
    let on = !off.contains(&tag(&id, &sheet.slug));
    let classes = format!(
        "flex h-[18px] w-[62px] items-center justify-center rounded-[4px] border text-[9px] {}",
        if on { "border-[#2f6fed] bg-[#1b2b48] text-[#9fc0ff]" } else { "border-border bg-[#11151b] text-faint" }
    );
    let tagged = tag(&id, &sheet.slug);
    let day = sheet.day.clone();
    return <span key={tag(&id, &sheet.slug)} className="w-[78px]">
        <button className={classes} onClick={move |_| flip(tagged.clone(), name.clone(), day.clone())}>
            {if on { "on" } else { "off" }}
        </button>
    </span>;
}

fn flip(tagged: String, name: String, day: String) {
    let mut all = off.clone();
    if let Some(at) = all.iter().position(|held| held == &tagged) {
        all.remove(at);
        note = format!("{name} is back on {day}");
        remember(format!("was put back on {day}"), name);
    } else {
        all.push(tagged);
        note = format!("{name} comes off {day}");
        remember(format!("came off {day}"), name);
    }
    off = all;
}

fn remember(what: String, who: String) {
    let mut all = activity.clone();
    all.insert(0, Happened { who, what, when_said: "just now".to_string(), week: false });
    all.truncate(14);
    activity = all;
}

fn tag(id: &str, slug: &str) -> String {
    format!("{id}/{slug}")
}
