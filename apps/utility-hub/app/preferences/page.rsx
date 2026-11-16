// PREFERENCES — the only screen that is about the machine rather than
// the production.
//
// Everything here leaves the application: a real registry entry, the
// system's own theme, locale and timezone, a real terminal, a real
// notification, and a real file written where a dialog puts it.

let mut departments: Vec<Department> = Vec::new();
let mut note: String = String::new();
let mut autostart_on: bool = false;
let mut machine: Vec<(String, String)> = Vec::new();

fn Page() -> Element {
    effect((), || {
        autostart_on = rustx::autostart::is_on("Reelhouse");
        machine = read_machine();
    });

    let people: usize = departments.iter().map(|department| department.crew.len()).sum();

    return <div key="preferences" className="px-5 py-4">
        <h1 className="text-[13px] font-semibold text-foreground">Preferences</h1>
        <p className="mt-0.5 text-[11px] text-muted">Everything on this screen is the machine's, not the production's.</p>

        <div className="mt-4 max-w-[560px] rounded-md border border-border bg-[#0f131a] p-4">
            <div className="flex items-center">
                <span>
                    <span className="block text-[12px] text-foreground">Start Reelhouse with the session</span>
                    <span className="block text-[11px] text-muted">
                        {if rustx::autostart::supported() {
                            "A real entry under the system's startup list."
                        } else {
                            "This platform does not offer one from here."
                        }}
                    </span>
                </span>
                <button className="ml-auto" onClick={move |_| flip_autostart()}>
                    {if autostart_on {
                        <span className="rounded-md bg-blue px-3 py-1 text-[11px] text-white">on</span>
                    } else {
                        <span className="rounded-md border border-border px-3 py-1 text-[11px] text-muted">off</span>
                    }}
                </button>
            </div>
        </div>

        <div className="mt-3 max-w-[560px] rounded-md border border-border bg-[#0f131a] p-4">
            <div className="text-[12px] text-foreground">This machine</div>
            <div className="mt-3 flex flex-col gap-2">
                {machine.iter().map(|fact| Fact(fact.0.clone(), fact.1.clone())).collect::<Vec<_>>()}
            </div>
            <div className="mt-4 flex gap-2">
                <button
                    className="rounded-md border border-border bg-[#11151b] px-3 py-[6px] text-[11px] text-foreground hover:border-faint"
                    onClick={move |_| open_a_terminal()}
                >
                    Open a terminal here
                </button>
                <button
                    className="rounded-md border border-border bg-[#11151b] px-3 py-[6px] text-[11px] text-foreground hover:border-faint"
                    onClick={move |_| machine = read_machine()}
                >
                    Read it again
                </button>
                <button
                    className="rounded-md border border-border bg-[#11151b] px-3 py-[6px] text-[11px] text-foreground hover:border-faint"
                    onClick={move |_| test_notification()}
                >
                    Send a test notification
                </button>
            </div>
        </div>

        <div className="mt-3 max-w-[560px] rounded-md border border-border bg-[#0f131a] p-4">
            <div className="text-[12px] text-foreground">The production, as this window holds it</div>
            <div className="mt-3 flex flex-col gap-2">
                {Fact("departments".to_string(), departments.len().to_string())}
                {Fact("people".to_string(), people.to_string())}
                {Fact("shooting days".to_string(), call_sheets().len().to_string())}
                {Fact("clips".to_string(), footage().len().to_string())}
            </div>
            <button
                className="mt-4 rounded-md bg-blue px-3 py-[6px] text-[11px] text-white hover:brightness-110"
                onClick={move |_| export_everything()}
            >
                Export the whole unit
            </button>
        </div>
    </div>;
}

fn Fact(name: String, value: String) -> Element {
    return <div key={name.clone()} className="flex items-baseline gap-3 text-[11px]">
        <span className="shrink-0 text-muted">{name}</span>
        <span className="ml-auto min-w-0 truncate text-right text-foreground">{value}</span>
    </div>;
}

fn flip_autostart() {
    let wanted = !autostart_on;
    match rustx::autostart::set(wanted, "Reelhouse") {
        Ok(()) => {
            autostart_on = rustx::autostart::is_on("Reelhouse");
            note = if autostart_on {
                "Reelhouse now starts with the session".to_string()
            } else {
                "Reelhouse no longer starts with the session".to_string()
            };
        }
        Err(error) => note = format!("the system refused: {error}"),
    }
}

fn open_a_terminal() {
    let at = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    match rustx::terminal::open(&at) {
        Ok(()) => note = format!("a terminal is open in {}", at.display()),
        Err(error) => note = format!("no terminal: {error}"),
    }
}

fn test_notification() {
    match rustx::notify::show(rustx::notify::What::new(
        "Reelhouse",
        "This is what a call-sheet notification looks like.",
    )) {
        Ok(()) => note = "a notification was raised".to_string(),
        Err(error) => note = format!("the notification failed: {error}"),
    }
}

/// Every department in one real file.
fn export_everything() {
    let ask = rustx::dialog::Ask::new("Export the whole unit")
        .filter("Comma separated", ["csv"])
        .named("reelhouse-unit.csv");
    let Ok(path) = rustx::dialog::save(ask) else {
        note = "the export was cancelled".to_string();
        return;
    };
    let mut text = String::from("department,name,email,role\n");
    for department in departments.iter() {
        for person in department.crew.iter() {
            text.push_str(&format!("{},{},{},{}\n", department.name, person.name, person.email, person.role));
        }
    }
    match std::fs::write(&path, text) {
        Ok(()) => {
            note = format!("the unit is at {}", path.display());
            let _ = rustx::notify::show(rustx::notify::What::new("Reelhouse", "The whole unit was exported."));
            let _ = rustx::os::shell::remember_recent(&path);
        }
        Err(error) => note = format!("the export failed: {error}"),
    }
}
