// ONE CALL SHEET — the day, and who is on it.
//
// The crew on the sheet is read out of the live department list, so a
// person imported this morning is on tonight's sheet. Export writes a
// real file where the system's own dialog puts it.

let mut departments: Vec<Department> = Vec::new();
let mut note: String = String::new();

fn Page(day: String) -> Element {
    let found = call_sheet_named(&day);

    return <div key={format!("sheet-{day}")} className="px-5 py-4">
        <div className="flex items-center gap-1.5 text-[11px] text-muted">
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/call-sheets".to_string())}>
                Call Sheets
            </button>
            <span className="text-faint">→</span>
            <span className="text-foreground">{found.as_ref().map(|sheet| sheet.day.clone()).unwrap_or_else(|| "Unknown day".to_string())}</span>
        </div>

        {match found {
            Some(sheet) => Sheet(sheet),
            None => Missing(day.clone()),
        }}
    </div>;
}

fn Sheet(sheet: CallSheet) -> Element {
    // Everyone whose department has anyone at all: a sheet names the
    // unit, and the unit is the departments the production has.
    let unit: Vec<(String, Person)> = departments
        .iter()
        .flat_map(|department| {
            department
                .crew
                .iter()
                .take(4)
                .map(|person| (department.name.clone(), person.clone()))
                .collect::<Vec<_>>()
        })
        .collect();
    let to_save = sheet.clone();

    return <div key={sheet.slug.clone()}>
        <div className="mt-3 flex items-start">
            <div>
                <h1 className="text-[16px] font-semibold text-foreground">{format!("{} · {}", sheet.day, sheet.date)}</h1>
                <p className="mt-1 text-[11px] text-muted">{format!("{} · {}", sheet.unit, sheet.location)}</p>
            </div>
            <button
                className="ml-auto flex items-center gap-1.5 rounded-md bg-blue px-2.5 py-[6px] text-[11px] text-white hover:brightness-110"
                onClick={move |_| save_sheet(to_save.clone())}
            >
                <Download size={11} color="#ffffff" />
                Export the sheet
            </button>
        </div>

        <div className="mt-4 grid grid-cols-4 gap-2.5">
            {Cell("CALL".to_string(), sheet.call.clone())}
            {Cell("WRAP".to_string(), sheet.wrap.clone())}
            {Cell("SCENES".to_string(), sheet.scenes.len().to_string())}
            {Cell("STATUS".to_string(), if sheet.locked { "locked".to_string() } else { "open".to_string() })}
        </div>

        <div className="mt-5 grid grid-cols-[1fr_1.1fr] gap-3">
            <div className="rounded-md border border-border bg-[#0f131a] p-4">
                <div className="text-[9px] tracking-[0.14em] text-faint">SCENES</div>
                <div className="mt-3 flex flex-col gap-1.5">
                    {sheet.scenes.iter().map(|scene| Scene(scene.clone())).collect::<Vec<_>>()}
                </div>
            </div>

            <div className="rounded-md border border-border bg-[#0f131a] p-4">
                <div className="text-[9px] tracking-[0.14em] text-faint">ON THE SHEET</div>
                <div className="mt-3 flex flex-col gap-1">
                    {unit.iter().map(|pair| Member(pair.0.clone(), pair.1.clone())).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    </div>;
}

fn Cell(name: String, value: String) -> Element {
    return <div key={name.clone()} className="rounded-md border border-border bg-[#0f131a] px-3.5 py-3">
        <div className="text-[9px] tracking-[0.12em] text-faint">{name}</div>
        <div className="mt-1.5 text-[16px] font-semibold tabular-nums text-foreground">{value}</div>
    </div>;
}

fn Scene(scene: String) -> Element {
    return <div key={scene.clone()} className="rounded-[4px] border border-[#243040] px-2.5 py-[6px] text-[11px] text-foreground">
        {scene}
    </div>;
}

fn Member(department: String, person: Person) -> Element {
    let going = format!("/crew/{}", person.id);
    return <button
        key={person.id.clone()}
        className="flex items-center gap-2 rounded-md px-1.5 py-[5px] text-left hover:bg-[#141922]"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span style={avatar_paint(&person.name, 18)}>{initials(&person.name)}</span>
        <span className="text-[11px] text-foreground">{person.name.clone()}</span>
        <span className="text-[10px] text-faint">{person.role.clone()}</span>
        <span className="ml-auto text-[10px] text-muted">{department}</span>
    </button>;
}

fn Missing(day: String) -> Element {
    return <div className="mt-4">
        <h1 className="text-[15px] font-semibold text-foreground">There is no sheet for that day</h1>
        <p className="mt-1.5 text-[11px] text-muted">
            {format!("Nothing on the schedule is called “{day}”.")}
        </p>
        <button
            className="mt-4 rounded-md bg-blue px-3 py-[6px] text-[11px] text-white hover:brightness-110"
            onClick={move |_| rustx::router::navigate("/call-sheets".to_string())}
        >
            The whole schedule
        </button>
    </div>;
}

fn save_sheet(sheet: CallSheet) {
    let ask = rustx::dialog::Ask::new("Export this call sheet")
        .filter("Plain text", ["txt"])
        .named(&format!("{}.txt", sheet.slug));
    let Ok(path) = rustx::dialog::save(ask) else {
        note = "the export was cancelled".to_string();
        return;
    };
    let text = format!(
        "{} ({})\n{}\n{}\ncall {} · wrap {}\n\nSCENES\n{}\n",
        sheet.day,
        sheet.date,
        sheet.unit,
        sheet.location,
        sheet.call,
        sheet.wrap,
        sheet.scenes.join("\n")
    );
    match std::fs::write(&path, text) {
        Ok(()) => {
            note = format!("{} written to {}", sheet.day, path.display());
            let _ = rustx::notify::show(rustx::notify::What::new(
                "Reelhouse",
                format!("{} exported.", sheet.day),
            ));
            let _ = rustx::os::shell::remember_recent(&path);
        }
        Err(error) => note = format!("the export failed: {error}"),
    }
}
