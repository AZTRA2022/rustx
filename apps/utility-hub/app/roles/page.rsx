// ROLES & ACCESS — the three roles the production uses, what each one
// is allowed to do, and who holds it.
//
// The permissions are the production's own policy. The counts and the
// names are read off the live department list, so a person imported
// this morning appears under their role here too.

let mut departments: Vec<Department> = Vec::new();
let mut note: String = String::new();
let mut granted: Vec<String> = Vec::new();

fn Page() -> Element {
    effect((), || {
        if granted.is_empty() {
            granted = starting_grants();
        }
    });

    let people: Vec<(String, Person)> = departments
        .iter()
        .flat_map(|department| {
            department
                .crew
                .iter()
                .map(|person| (department.name.clone(), person.clone()))
                .collect::<Vec<_>>()
        })
        .collect();

    return <div key="roles" className="px-5 py-4">
        <h1 className="text-[13px] font-semibold text-foreground">Roles &amp; Access</h1>
        <p className="mt-0.5 text-[11px] text-muted">
            What each role on this production may do, and who holds it.
        </p>

        <div className="mt-4 grid grid-cols-3 gap-2.5">
            {RoleCard("Lead".to_string(), people.clone())}
            {RoleCard("Operator".to_string(), people.clone())}
            {RoleCard("Trainee".to_string(), people.clone())}
        </div>

        <h2 className="mt-6 text-[9px] tracking-[0.14em] text-faint">WHAT EACH ROLE MAY DO</h2>
        <div className="mt-3 overflow-hidden rounded-md border border-border bg-[#0f131a]">
            <div className="grid grid-cols-[1fr_90px_90px_90px] border-b border-border px-3.5 py-2 text-[10px] tracking-[0.1em] text-faint">
                <span>PERMISSION</span>
                <span className="text-center">LEAD</span>
                <span className="text-center">OPERATOR</span>
                <span className="text-center">TRAINEE</span>
            </div>
            {permissions().iter().map(|name| PermissionRow(name.clone())).collect::<Vec<_>>()}
        </div>
        <p className="mt-3 text-[10px] text-faint">
            A cell is a switch. The policy lives for as long as this window does, which is as far as
            a local demo can honestly take it.
        </p>
    </div>;
}

fn RoleCard(role: String, people: Vec<(String, Person)>) -> Element {
    let held: Vec<(String, Person)> = people.into_iter().filter(|pair| pair.1.role == role).collect();
    let badge = format!(
        "border-radius:4px;padding:2px 9px;font-size:11px;background-color:{};color:{}",
        role_background(&role),
        role_ink(&role)
    );
    let given = granted.clone();
    let may = permissions()
        .into_iter()
        .filter(|name| given.contains(&tag(name, &role)))
        .count();

    return <div key={role.clone()} className="rounded-md border border-border bg-[#0f131a] p-3.5">
        <div className="flex items-center gap-2">
            <span style={badge}>{role.clone()}</span>
            <span className="ml-auto text-[10px] text-faint">{format!("{} people", held.len())}</span>
        </div>
        <div className="mt-2.5 text-[11px] text-muted">{format!("{may} of {} permissions", permissions().len())}</div>
        <div className="mt-2.5 flex flex-wrap gap-1.5">
            {held.iter().take(10).map(|pair| Face(pair.1.clone())).collect::<Vec<_>>()}
            {if held.len() > 10 {
                <span className="self-center text-[10px] text-faint">{format!("+{}", held.len() - 10)}</span>
            } else {
                <span></span>
            }}
        </div>
    </div>;
}

fn Face(person: Person) -> Element {
    let going = format!("/crew/{}", person.id);
    return <button key={person.id.clone()} onClick={move |_| rustx::router::navigate(going.clone())}>
        <span style={avatar_paint(&person.name, 20)}>{initials(&person.name)}</span>
    </button>;
}

fn PermissionRow(name: String) -> Element {
    return <div key={name.clone()} className="grid grid-cols-[1fr_90px_90px_90px] items-center border-b border-border px-3.5 py-2 text-[11px] last:border-b-0">
        <span className="text-foreground">{name.clone()}</span>
        {Cell(name.clone(), "Lead".to_string())}
        {Cell(name.clone(), "Operator".to_string())}
        {Cell(name.clone(), "Trainee".to_string())}
    </div>;
}

fn Cell(name: String, role: String) -> Element {
    let on = granted.contains(&tag(&name, &role));
    let classes = format!(
        "mx-auto flex h-[18px] w-[18px] items-center justify-center rounded-[4px] border {}",
        if on { "border-[#2f6fed] bg-[#1b2b48]" } else { "border-border bg-[#11151b]" }
    );
    let tagged = tag(&name, &role);
    return <button key={tag(&name, &role)} className={classes} onClick={move |_| flip(tagged.clone())}>
        {if on {
            <Check size={11} color="#6ea0ff" />
        } else {
            <span></span>
        }}
    </button>;
}

fn flip(tagged: String) {
    let mut all = granted.clone();
    if let Some(at) = all.iter().position(|held| held == &tagged) {
        all.remove(at);
        note = format!("{tagged} revoked");
    } else {
        all.push(tagged.clone());
        note = format!("{tagged} granted");
    }
    granted = all;
}

fn tag(permission: &str, role: &str) -> String {
    format!("{permission} / {role}")
}

fn permissions() -> Vec<String> {
    vec![
        "See the call sheet".to_string(),
        "Change a call time".to_string(),
        "Sign kit out".to_string(),
        "Import a crew list".to_string(),
        "Export the roster".to_string(),
        "Approve a budget line".to_string(),
        "Lock a shooting day".to_string(),
    ]
}

/// Who may do what when the application opens: everything for a lead,
/// the floor for an operator, the basics for a trainee.
fn starting_grants() -> Vec<String> {
    let mut grants = Vec::new();
    for permission in permissions() {
        grants.push(tag(&permission, "Lead"));
    }
    for permission in ["See the call sheet", "Sign kit out", "Export the roster"] {
        grants.push(tag(permission, "Operator"));
    }
    grants.push(tag("See the call sheet", "Trainee"));
    grants
}
