// SUPPORT — the left-hand link in the footer.
//
// A support page on a machine with no network cannot file a ticket.
// What it can do is collect everything a support engineer would ask
// for — the application's own counts and the machine's own facts —
// and put it on the clipboard or in a file, which is what this page
// does.

let mut departments: Vec<Department> = Vec::new();
let mut note: String = String::new();
let mut machine: Vec<(String, String)> = Vec::new();

fn Page() -> Element {
    effect((), || {
        machine = read_machine();
    });

    return <div key="support" className="px-5 py-4">
        <h1 className="text-[13px] font-semibold text-foreground">Support</h1>
        <p className="mt-0.5 max-w-[560px] text-[11px] leading-[1.7] text-muted">
            There is no ticket queue behind this window and this page will not pretend there is.
            What it does instead is gather the report a support engineer would ask you for.
        </p>

        <div className="mt-4 max-w-[560px] rounded-md border border-border bg-[#0f131a] p-4">
            <div className="text-[9px] tracking-[0.14em] text-faint">THE REPORT</div>
            <pre id="report" className="mt-3 max-h-56 overflow-auto whitespace-pre-wrap rounded-md border border-border bg-[#0b0e12] px-3 py-2.5 text-[10px] leading-[1.6] text-muted">
                {report()}
            </pre>
            <div className="mt-3 flex gap-2">
                <button
                    className="rounded-md bg-blue px-3 py-[6px] text-[11px] text-white hover:brightness-110"
                    onClick={move |_| copy_report()}
                >
                    Copy the report
                </button>
                <button
                    className="rounded-md border border-border bg-[#11151b] px-3 py-[6px] text-[11px] text-foreground hover:border-faint"
                    onClick={move |_| save_report()}
                >
                    Save it to a file
                </button>
            </div>
        </div>

        <div className="mt-3 max-w-[560px] rounded-md border border-border bg-[#0f131a] p-4">
            <div className="text-[9px] tracking-[0.14em] text-faint">BEFORE YOU ASK</div>
            <div className="mt-3 flex flex-col gap-1">
                {Link("Departments and crew".to_string(), "/help/departments".to_string())}
                {Link("Importing and exporting".to_string(), "/help/import-export".to_string())}
                {Link("Call sheets and notifications".to_string(), "/help/notifications".to_string())}
                {Link("Preferences and this machine".to_string(), "/help/the-machine".to_string())}
            </div>
        </div>
    </div>;
}

fn Link(title: String, route: String) -> Element {
    let going = route.clone();
    return <button
        key={route.clone()}
        className="flex items-center gap-1.5 rounded-md px-2 py-[6px] text-left text-[11px] text-muted hover:bg-[#141922] hover:text-foreground"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        {title}
        <ChevronRight size={10} color="#5a6572" />
    </button>;
}

fn report() -> String {
    let people: usize = departments.iter().map(|department| department.crew.len()).sum();
    let mut text = format!(
        "Reelhouse v2.3.1\n{} departments, {people} people\n{} shooting days, {} clips\n",
        departments.len(),
        call_sheets().len(),
        footage().len()
    );
    for fact in machine.iter() {
        text.push_str(&format!("{}: {}\n", fact.0, fact.1));
    }
    text
}

fn copy_report() {
    match rustx::clipboard::set_text(&report()) {
        Ok(()) => note = "the report is on the clipboard".to_string(),
        Err(error) => note = format!("the clipboard refused: {error}"),
    }
}

fn save_report() {
    let ask = rustx::dialog::Ask::new("Save the support report")
        .filter("Plain text", ["txt"])
        .named("reelhouse-support.txt");
    let Ok(path) = rustx::dialog::save(ask) else {
        note = "the report was not saved".to_string();
        return;
    };
    match std::fs::write(&path, report()) {
        Ok(()) => {
            note = format!("the report is at {}", path.display());
            let _ = rustx::os::shell::remember_recent(&path);
        }
        Err(error) => note = format!("the report could not be written: {error}"),
    }
}
