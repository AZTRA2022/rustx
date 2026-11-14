// SEARCH — where Enter in the top bar lands.
//
// The term is in the address (`/search?q=…`) rather than in a value,
// so a result can be gone back to and a search can be reloaded. The
// three lists are the live ones: crew, departments and call sheets.

let mut departments: Vec<Department> = Vec::new();

fn Page() -> Element {
    let term = rustx::router::query("q").replace('+', " ");
    let needle = term.to_lowercase();

    let people: Vec<(String, Person)> = if needle.is_empty() {
        Vec::new()
    } else {
        departments
            .iter()
            .flat_map(|department| {
                department
                    .crew
                    .iter()
                    .filter(|person| {
                        person.name.to_lowercase().contains(&needle)
                            || person.email.to_lowercase().contains(&needle)
                            || person.role.to_lowercase().contains(&needle)
                    })
                    .map(|person| (department.clone().name, person.clone()))
                    .collect::<Vec<_>>()
            })
            .collect()
    };

    let units: Vec<Department> = if needle.is_empty() {
        Vec::new()
    } else {
        departments
            .iter()
            .filter(|department| department.name.to_lowercase().contains(&needle))
            .cloned()
            .collect()
    };

    let sheets: Vec<CallSheet> = if needle.is_empty() {
        Vec::new()
    } else {
        call_sheets()
            .into_iter()
            .filter(|sheet| {
                sheet.day.to_lowercase().contains(&needle)
                    || sheet.location.to_lowercase().contains(&needle)
                    || sheet.unit.to_lowercase().contains(&needle)
                    || sheet.scenes.iter().any(|scene| scene.to_lowercase().contains(&needle))
            })
            .collect()
    };

    let found = people.len() + units.len() + sheets.len();

    return <div key={format!("search-{term}")} className="px-5 py-4">
        <h1 className="text-[13px] font-semibold text-foreground">
            {if term.is_empty() { "Search".to_string() } else { format!("“{term}”") }}
        </h1>
        <p className="mt-0.5 text-[11px] text-muted">
            {if term.is_empty() {
                "Type in the bar above and press Enter.".to_string()
            } else {
                format!("{found} result(s) across crew, departments and call sheets.")
            }}
        </p>

        {if units.is_empty() {
            <span></span>
        } else {
            <div className="mt-4">
                <div className="text-[9px] tracking-[0.14em] text-faint">DEPARTMENTS</div>
                <div className="mt-2 flex flex-col gap-1">
                    {units.iter().map(|department| UnitRow(department.clone())).collect::<Vec<_>>()}
                </div>
            </div>
        }}

        {if people.is_empty() {
            <span></span>
        } else {
            <div className="mt-5">
                <div className="text-[9px] tracking-[0.14em] text-faint">CREW</div>
                <div className="mt-2 flex flex-col gap-1">
                    {people.iter().map(|pair| PersonRow(pair.0.clone(), pair.1.clone())).collect::<Vec<_>>()}
                </div>
            </div>
        }}

        {if sheets.is_empty() {
            <span></span>
        } else {
            <div className="mt-5">
                <div className="text-[9px] tracking-[0.14em] text-faint">CALL SHEETS</div>
                <div className="mt-2 flex flex-col gap-1">
                    {sheets.iter().map(|sheet| SheetRow(sheet.clone())).collect::<Vec<_>>()}
                </div>
            </div>
        }}

        {if found == 0 && !term.is_empty() {
            <p className="mt-5 text-[11px] text-faint">Nothing on this production matches that.</p>
        } else {
            <span></span>
        }}
    </div>;
}

fn UnitRow(department: Department) -> Element {
    let going = format!("/departments/{}", department.slug);
    return <button
        key={department.slug.clone()}
        className="flex items-center gap-2 rounded-md border border-border bg-[#0f131a] px-3 py-2 text-left hover:border-faint"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <Network size={12} color="#6ea0ff" />
        <span className="text-[12px] text-foreground">{department.name.clone()}</span>
        <span className="ml-auto text-[10px] text-faint">{format!("{} members", department.crew.len())}</span>
    </button>;
}

fn PersonRow(department: String, person: Person) -> Element {
    let going = format!("/crew/{}", person.id);
    return <button
        key={person.id.clone()}
        className="flex items-center gap-2.5 rounded-md border border-border bg-[#0f131a] px-3 py-2 text-left hover:border-faint"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span style={avatar_paint(&person.name, 20)}>{initials(&person.name)}</span>
        <span className="text-[12px] text-foreground">{person.name.clone()}</span>
        <span className="text-[10px] text-muted">{person.email.clone()}</span>
        <span className="ml-auto text-[10px] text-faint">{format!("{department} · {}", person.role)}</span>
    </button>;
}

fn SheetRow(sheet: CallSheet) -> Element {
    let going = format!("/call-sheets/{}", sheet.slug);
    return <button
        key={sheet.slug.clone()}
        className="flex items-center gap-2 rounded-md border border-border bg-[#0f131a] px-3 py-2 text-left hover:border-faint"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <Folder size={12} color="#6ea0ff" />
        <span className="text-[12px] text-foreground">{sheet.day.clone()}</span>
        <span className="text-[10px] text-muted">{sheet.location.clone()}</span>
        <span className="ml-auto text-[10px] tabular-nums text-faint">{format!("call {}", sheet.call)}</span>
    </button>;
}
