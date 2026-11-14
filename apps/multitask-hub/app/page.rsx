// MultiTask Hub.
//
// Several `Project`s, each its own `Vec<Task>`, and one roll-up at
// the top that reads across every project at once — the "solve
// orchestration problems across many projects" of the showcase copy,
// scoped to what a dashboard without a backend can really show.

#[derive(Clone)]
struct Task {
    title: String,
    done: bool,
    blocked: bool,
}

#[derive(Clone)]
struct Project {
    name: String,
    tasks: Vec<Task>,
}

let mut projects: Vec<Project> = vec![
    Project {
        name: "launch".to_string(),
        tasks: vec![
            Task { title: "write the landing page".to_string(), done: true, blocked: false },
            Task { title: "wire up billing".to_string(), done: false, blocked: true },
            Task { title: "record the demo".to_string(), done: false, blocked: false },
        ],
    },
    Project {
        name: "migration".to_string(),
        tasks: vec![
            Task { title: "export the old schema".to_string(), done: true, blocked: false },
            Task { title: "backfill the new tables".to_string(), done: false, blocked: false },
        ],
    },
    Project {
        name: "support".to_string(),
        tasks: vec![
            Task { title: "triage the backlog".to_string(), done: false, blocked: true },
        ],
    },
];
let mut active: usize = 0;
let mut draft = String::new();

fn Page() -> Element {
    return <div style={{ padding: "24px" }}>
        <p style={{ margin: "0 0 18px", fontSize: "13px", color: "#9a9db3" }}>
            orchestration across {projects.len()} projects at once.
        </p>

        {RollUp()}
        {Tabs()}
        {Board()}
    </div>;
}

fn RollUp() -> Element {
    let mut total = 0;
    let mut done = 0;
    let mut blocked = 0;
    for project in projects.iter() {
        for task in project.tasks.iter() {
            total += 1;
            if task.done { done += 1; }
            if task.blocked && !task.done { blocked += 1; }
        }
    }

    return <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "12px", marginBottom: "20px" }}>
        {Stat("total tasks".to_string(), total.to_string(), "#60a5fa".to_string())}
        {Stat("done".to_string(), done.to_string(), "#34d399".to_string())}
        {Stat("blocked".to_string(), blocked.to_string(), "#f87171".to_string())}
    </div>;
}

fn Stat(label: String, value: String, color: String) -> Element {
    // A `style={{ … }}` object is read at compile time and takes only
    // literal strings; a colour decided here is written as the
    // declaration itself.
    let number = format!("font-size: 26px; font-weight: 700; color: {}", color);

    return <div style={{ border: "1px solid #1d2230", borderRadius: "10px", padding: "14px", backgroundColor: "#121420" }}>
        <div style={{ fontSize: "11px", color: "#8a8fa8", textTransform: "uppercase", letterSpacing: "1px" }}>{label}</div>
        <div style={number}>{value}</div>
    </div>;
}

fn Tabs() -> Element {
    return <div id="tabs" style={{ display: "flex", gap: "8px", marginBottom: "16px" }}>
        {projects.iter().enumerate().map(|(index, project)| ProjectTab(index, project.clone())).collect::<Vec<_>>()}
    </div>;
}

fn ProjectTab(index: usize, project: Project) -> Element {
    let is_active = active == index;
    let done = project.tasks.iter().filter(|task| task.done).count();
    let total = project.tasks.len();
    let tab = format!(
        "padding: 8px 14px; border-radius: 8px; color: #e7e8ee; cursor: pointer; \
         border: 1px solid {}; background-color: {}",
        if is_active { "#6366f1" } else { "#1d2230" },
        if is_active { "#1d2040" } else { "#121420" }
    );

    return <button key={index.to_string()} onClick={move |_| active = index} style={tab}>
        {project.name.clone()} <span style={{ color: "#8a8fa8" }}>{format!("{}/{}", done, total)}</span>
    </button>;
}

fn Board() -> Element {
    if projects.is_empty() {
        return <p>no projects.</p>;
    }
    let index = active.min(projects.len() - 1);
    let project = projects[index].clone();
    let route_name = project.name.clone();

    return <div style={{ border: "1px solid #1d2230", borderRadius: "10px", padding: "16px", backgroundColor: "#121420" }}>
        <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: "8px" }}>
            <button id="open-project" onClick={move |_| rustx::router::navigate(format!("/project/{}", route_name))} style={{ padding: "5px 10px", fontSize: "12px", backgroundColor: "transparent", color: "#9a9db3", border: "1px solid #1d2230", borderRadius: "6px", cursor: "pointer" }}>
                open project page →
            </button>
        </div>
        <ul id="tasks" style={{ listStyle: "none", margin: "0 0 14px", padding: "0", display: "flex", flexDirection: "column", gap: "6px" }}>
            {project.tasks.iter().enumerate().map(|(task_index, task)| TaskRow(index, task_index, task.clone())).collect::<Vec<_>>()}
        </ul>

        <div style={{ display: "flex", gap: "8px" }}>
            <input id="draft" placeholder="a new task" value={draft.clone()}
                onInput={move |event| draft = event.value}
                onKeyDown={move |event| if event.key == "Enter" { add_task(index); }}
                style={{ flex: "1", padding: "7px", backgroundColor: "#0b0c10", border: "1px solid #1d2230", color: "#e7e8ee", borderRadius: "6px" }} />
            <button onClick={move |_| add_task(index)} style={{ padding: "7px 14px", backgroundColor: "#4338ca", color: "#fff", border: "none", borderRadius: "6px", cursor: "pointer" }}>add</button>
        </div>
    </div>;
}

fn TaskRow(project_index: usize, task_index: usize, task: Task) -> Element {
    let row = format!(
        "display: flex; align-items: center; gap: 10px; padding: 8px 10px; \
         border-radius: 6px; background-color: {}",
        if task.blocked && !task.done { "#241319" } else { "#0e1018" }
    );
    let title = format!(
        "flex: 1; text-decoration: {}; color: {}",
        if task.done { "line-through" } else { "none" },
        if task.done { "#6b7086" } else { "#e7e8ee" }
    );

    return <li key={task_index.to_string()} style={row}>
        <input type="checkbox" checked={task.done}
            onChange={move |_| toggle_done(project_index, task_index)} />
        <span style={title}>
            {task.title.clone()}
        </span>
        {if task.blocked && !task.done { BlockedTag() } else { <span></span> }}
        <button onClick={move |_| toggle_blocked(project_index, task_index)} style={{ padding: "3px 8px", fontSize: "11px", backgroundColor: "transparent", color: "#8a8fa8", border: "1px solid #1d2230", borderRadius: "5px", cursor: "pointer" }}>
            {if task.blocked { "unblock" } else { "block" }}
        </button>
    </li>;
}

fn BlockedTag() -> Element {
    return <span style={{ fontSize: "10px", color: "#f87171", border: "1px solid #4a1c2e", borderRadius: "999px", padding: "2px 8px" }}>blocked</span>;
}

fn add_task(project_index: usize) {
    if draft.trim().is_empty() {
        return;
    }
    // The copy, the change, and the copy put back. A page value hands
    // back a copy when it is read, so `projects.get_mut(…)` changed a
    // copy that was dropped at the end of the statement: the task was
    // never added, and nothing said so.
    let mut all = projects.clone();
    if let Some(project) = all.get_mut(project_index) {
        project.tasks.push(Task { title: draft.trim().to_string(), done: false, blocked: false });
    }
    projects = all;
    draft = String::new();
}

fn toggle_done(project_index: usize, task_index: usize) {
    let mut all = projects.clone();
    if let Some(project) = all.get_mut(project_index)
        && let Some(task) = project.tasks.get_mut(task_index)
    {
        task.done = !task.done;
    }
    projects = all;
}

fn toggle_blocked(project_index: usize, task_index: usize) {
    let mut all = projects.clone();
    if let Some(project) = all.get_mut(project_index)
        && let Some(task) = project.tasks.get_mut(task_index)
    {
        task.blocked = !task.blocked;
    }
    projects = all;
}
