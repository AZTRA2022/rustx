// CALL TIMES — the same roster, read against the day's call.
//
// The call of the unit comes from the first sheet that is not locked,
// which is the next day to shoot; each person's own call is offset
// from it by their role, the way a real sheet staggers a unit.

let mut departments: Vec<Department> = Vec::new();
let mut note: String = String::new();
let mut shifted: Vec<String> = Vec::new();

fn Page(slug: String) -> Element {
    let crew = departments
        .iter()
        .find(|held| held.slug == slug)
        .map(|held| held.crew.clone())
        .unwrap_or_default();
    let next = call_sheets().into_iter().find(|sheet| !sheet.locked);
    let unit_call = next.as_ref().map(|sheet| sheet.call.clone()).unwrap_or_else(|| "07:00".to_string());
    let day = next.as_ref().map(|sheet| sheet.day.clone()).unwrap_or_else(|| "the next day".to_string());
    let going = next.as_ref().map(|sheet| format!("/call-sheets/{}", sheet.slug)).unwrap_or_else(|| "/call-sheets".to_string());

    return <div key={format!("times-{slug}")}>
        <div className="flex items-center rounded-md border border-border bg-[#0f131a] px-3 py-2.5">
            <span className="text-[11px] text-muted">
                {format!("Unit call for {day} is {unit_call}. A person's own call is staggered by what they do.")}
            </span>
            <button
                className="ml-auto shrink-0 rounded-md border border-border bg-[#11151b] px-2.5 py-[5px] text-[10px] text-foreground hover:border-faint"
                onClick={move |_| rustx::router::navigate(going.clone())}
            >
                Open the sheet
            </button>
        </div>

        <div className="mt-3 flex flex-col gap-1.5">
            {crew
                .iter()
                .enumerate()
                .map(|(index, person)| Row(index, person.clone(), unit_call.clone()))
                .collect::<Vec<_>>()}
        </div>
    </div>;
}

fn Row(index: usize, person: Person, unit_call: String) -> Element {
    let moved = shifted.contains(&person.id);
    let own = stagger(&unit_call, &person.role, index, moved);
    let going = format!("/crew/{}", person.id);
    let id = person.id.clone();

    return <div key={person.id.clone()} className="flex items-center gap-2.5 rounded-md border border-border bg-[#0f131a] px-3 py-2">
        <button className="flex items-center gap-2.5" onClick={move |_| rustx::router::navigate(going.clone())}>
            <span style={avatar_paint(&person.name, 20)}>{initials(&person.name)}</span>
            <span className="text-[12px]">{person.name.clone()}</span>
        </button>
        <span className="text-[10px] text-faint">{person.role.clone()}</span>
        <span className="ml-auto text-[11px] tabular-nums text-muted">
            {if moved { format!("call {own} · moved") } else { format!("call {own}") }}
        </span>
        <button
            className="shrink-0 rounded-md border border-border bg-[#11151b] px-2 py-[4px] text-[10px] text-muted hover:text-foreground"
            onClick={move |_| shift(id.clone(), person.name.clone())}
        >
            {if moved { "unmove" } else { "move 30m" }}
        </button>
    </div>;
}

/// Thirty minutes earlier for the leads, the unit call for everyone
/// else, and a few minutes of stagger so a roster does not read as one
/// time repeated.
fn stagger(unit_call: &str, role: &str, index: usize, moved: bool) -> String {
    let minutes = minutes_of(unit_call);
    let shift = match role {
        "Lead" => -30i64,
        "Trainee" => 15,
        _ => (index as i64 % 3) * 5,
    };
    spell(minutes + shift + if moved { 30 } else { 0 })
}

fn minutes_of(clock: &str) -> i64 {
    let mut parts = clock.split(':');
    let hours: i64 = parts.next().unwrap_or("7").parse().unwrap_or(7);
    let minutes: i64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    hours * 60 + minutes
}

fn spell(minutes: i64) -> String {
    let wrapped = ((minutes % 1440) + 1440) % 1440;
    format!("{:02}:{:02}", wrapped / 60, wrapped % 60)
}

fn shift(id: String, name: String) {
    let mut all = shifted.clone();
    if let Some(at) = all.iter().position(|held| held == &id) {
        all.remove(at);
        note = format!("{name} is back on the unit call");
    } else {
        all.push(id);
        note = format!("{name} moves half an hour");
    }
    shifted = all;
}
