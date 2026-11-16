// DEPARTMENTS — the index behind "All Departments" in the pane, and
// the page the rail's Departments item lands on when no unit is
// chosen yet.

let mut departments: Vec<Department> = Vec::new();
let mut dept_filter: String = String::new();

fn Page() -> Element {
    let needle = dept_filter.to_lowercase();
    let shown: Vec<Department> = departments
        .iter()
        .filter(|department| needle.is_empty() || department.name.to_lowercase().contains(&needle))
        .cloned()
        .collect();
    let people: usize = departments.iter().map(|department| department.crew.len()).sum();

    return <div key="departments" className="px-5 py-4">
        <h1 className="text-[13px] font-semibold text-foreground">All Departments</h1>
        <p className="mt-0.5 text-[11px] text-muted">
            {format!("{} departments, {people} people. Choose one to open its roster.", departments.len())}
        </p>

        <div id="department-cards" className="mt-4 grid grid-cols-2 gap-2.5">
            {shown.iter().map(|department| Card(department.clone())).collect::<Vec<_>>()}
        </div>

        {if shown.is_empty() {
            <p className="mt-4 text-[11px] text-faint">Nothing matches that filter.</p>
        } else {
            <span></span>
        }}
    </div>;
}

fn Card(department: Department) -> Element {
    let leads = department.crew.iter().filter(|person| person.role == "Lead").count();
    let trainees = department.crew.iter().filter(|person| person.role == "Trainee").count();
    let going = format!("/departments/{}", department.slug);

    return <button
        key={department.slug.clone()}
        className="rounded-md border border-border bg-[#0f131a] p-3.5 text-left hover:border-faint"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="flex items-center gap-2">
            <Network size={13} color="#6ea0ff" />
            <span className="text-[12px] font-semibold text-foreground">{department.name.clone()}</span>
            <span className="ml-auto text-[10px] text-faint">{format!("{} members", department.crew.len())}</span>
        </span>
        <span className="mt-2.5 flex flex-wrap gap-1.5">
            {department.crew.iter().take(6).map(|person| Face(person.clone())).collect::<Vec<_>>()}
            {if department.crew.len() > 6 {
                <span className="self-center text-[10px] text-faint">{format!("+{}", department.crew.len() - 6)}</span>
            } else {
                <span></span>
            }}
        </span>
        <span className="mt-2.5 block text-[10px] text-muted">
            {format!("{leads} lead(s) · {trainees} trainee(s)")}
        </span>
    </button>;
}

fn Face(person: Person) -> Element {
    return <span key={person.id.clone()} style={avatar_paint(&person.name, 20)}>{initials(&person.name)}</span>;
}
