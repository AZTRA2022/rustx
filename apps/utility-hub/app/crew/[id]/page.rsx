// ONE PERSON — what a row of a roster opens.
//
// Everything on this page is read back out of the department list the
// root layout holds, so a person imported from a CSV two minutes ago
// has a page here like anyone else.

let mut departments: Vec<Department> = Vec::new();
let mut activity: Vec<Happened> = Vec::new();
let mut note: String = String::new();

fn Page(id: String) -> Element {
    let found = departments.iter().find_map(|department| {
        department
            .crew
            .iter()
            .find(|person| person.id == id)
            .map(|person| (department.clone(), person.clone()))
    });

    return <div key={format!("person-{id}")} className="px-5 py-4">
        {match found {
            Some(pair) => Profile(pair.0, pair.1),
            None => Missing(id.clone()),
        }}
    </div>;
}

fn Profile(department: Department, person: Person) -> Element {
    let badge = format!(
        "border-radius:4px;padding:2px 9px;font-size:11px;background-color:{};color:{}",
        role_background(&person.role),
        role_ink(&person.role)
    );
    let back = format!("/departments/{}", department.slug);
    let mates: Vec<Person> = department
        .crew
        .iter()
        .filter(|other| other.id != person.id)
        .take(8)
        .cloned()
        .collect();

    let to_copy = person.email.clone();
    let to_ping = person.name.clone();
    let to_card = person.clone();

    return <div key={person.id.clone()}>
        <div className="flex items-center gap-1.5 text-[11px] text-muted">
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/departments".to_string())}>
                Departments
            </button>
            <span className="text-faint">→</span>
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate(back.clone())}>
                {department.name.clone()}
            </button>
            <span className="text-faint">→</span>
            <span className="text-foreground">{person.name.clone()}</span>
        </div>

        <div className="mt-4 flex items-center gap-3.5">
            <span style={avatar_paint(&person.name, 52)}>{initials(&person.name)}</span>
            <div>
                <h1 className="text-[16px] font-semibold text-foreground">{person.name.clone()}</h1>
                <div className="mt-1 flex items-center gap-2">
                    <span style={badge}>{person.role.clone()}</span>
                    <span className="text-[11px] text-muted">{person.email.clone()}</span>
                </div>
            </div>
            <div className="ml-auto flex items-center gap-2">
                <button
                    className="flex items-center gap-1.5 rounded-md border border-border bg-[#11151b] px-2.5 py-[6px] text-[11px] text-foreground hover:border-faint"
                    onClick={move |_| copy_email(to_copy.clone())}
                >
                    <Copy size={11} color="#c9d1d9" />
                    Copy address
                </button>
                <button
                    className="flex items-center gap-1.5 rounded-md border border-border bg-[#11151b] px-2.5 py-[6px] text-[11px] text-foreground hover:border-faint"
                    onClick={move |_| ping(to_ping.clone())}
                >
                    <Mail size={11} color="#c9d1d9" />
                    Send the call sheet
                </button>
                <button
                    className="flex items-center gap-1.5 rounded-md bg-blue px-2.5 py-[6px] text-[11px] text-white hover:brightness-110"
                    onClick={move |_| save_card(to_card.clone())}
                >
                    <Download size={11} color="#ffffff" />
                    Save card
                </button>
            </div>
        </div>

        <div className="mt-5 grid grid-cols-2 gap-3">
            <div className="rounded-md border border-border bg-[#0f131a] p-4">
                <div className="text-[9px] tracking-[0.14em] text-faint">ON THE UNIT</div>
                <div className="mt-3 flex flex-col gap-2">
                    {Fact("Department".to_string(), department.name.clone())}
                    {Fact("Role".to_string(), person.role.clone())}
                    {Fact("Address".to_string(), person.email.clone())}
                    {Fact("Handle".to_string(), person.id.clone())}
                    {Fact("Unit size".to_string(), format!("{} people", department.crew.len()))}
                </div>
            </div>

            <div className="rounded-md border border-border bg-[#0f131a] p-4">
                <div className="text-[9px] tracking-[0.14em] text-faint">ALSO IN THIS DEPARTMENT</div>
                <div className="mt-3 flex flex-col gap-1">
                    {mates.iter().map(|mate| Mate(mate.clone())).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    </div>;
}

fn Fact(name: String, value: String) -> Element {
    return <div key={name.clone()} className="flex items-baseline gap-3 text-[11px]">
        <span className="shrink-0 text-muted">{name}</span>
        <span className="ml-auto min-w-0 truncate text-right text-foreground">{value}</span>
    </div>;
}

fn Mate(person: Person) -> Element {
    let going = format!("/crew/{}", person.id);
    return <button
        key={person.id.clone()}
        className="flex items-center gap-2 rounded-md px-1.5 py-[5px] text-left hover:bg-[#141922]"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span style={avatar_paint(&person.name, 18)}>{initials(&person.name)}</span>
        <span className="text-[11px] text-foreground">{person.name.clone()}</span>
        <span className="ml-auto text-[10px] text-faint">{person.role.clone()}</span>
    </button>;
}

fn Missing(id: String) -> Element {
    return <div>
        <h1 className="text-[15px] font-semibold text-foreground">Nobody on this production has that handle</h1>
        <p className="mt-1.5 text-[11px] text-muted">
            {format!("There is no crew member called “{id}”. Choose a department on the left and open a roster.")}
        </p>
        <button
            className="mt-4 rounded-md bg-blue px-3 py-[6px] text-[11px] text-white hover:brightness-110"
            onClick={move |_| rustx::router::navigate("/departments".to_string())}
        >
            All departments
        </button>
    </div>;
}

// ---- what the buttons do ---------------------------------------------

fn copy_email(email: String) {
    match rustx::clipboard::set_text(&email) {
        Ok(()) => note = format!("{email} is on the clipboard"),
        Err(error) => note = format!("the clipboard refused: {error}"),
    }
}

fn ping(who: String) {
    match rustx::notify::show(rustx::notify::What::new(
        "Reelhouse",
        format!("Call sheet sent to {who}."),
    )) {
        Ok(()) => note = format!("a notification went out about {who}"),
        Err(error) => note = format!("the notification failed: {error}"),
    }
    let mut all = activity.clone();
    all.insert(0, Happened { who, what: "was sent the call sheet".to_string(), when_said: "just now".to_string(), week: false });
    all.truncate(14);
    activity = all;
}

/// A real file, where the system's own dialog puts it.
fn save_card(person: Person) {
    let ask = rustx::dialog::Ask::new("Save this crew card")
        .filter("Comma separated", ["csv"])
        .named(format!("{}.csv", person.id));
    let Ok(path) = rustx::dialog::save(ask) else {
        note = "the card was not saved".to_string();
        return;
    };
    let text = format!("name,email,role\n{},{},{}\n", person.name, person.email, person.role);
    match std::fs::write(&path, text) {
        Ok(()) => {
            note = format!("the card is at {}", path.display());
            let _ = rustx::os::shell::remember_recent(&path);
        }
        Err(error) => note = format!("the card could not be written: {error}"),
    }
}
