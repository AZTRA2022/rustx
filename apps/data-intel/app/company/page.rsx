// COMPANY — who is behind the layer, and what this window actually is.
//
// The last card on this page is the one that matters: a showcase
// application saying so, in the same type as everything else.

let mut tables: Vec<Dataset> = Vec::new();
let mut imported: u32 = 0;
let mut status: String = String::new();

let mut machine: Vec<(String, String)> = Vec::new();

fn Page() -> Element {
    effect((), || {
        machine = read_machine();
    });

    return <div key="company" id="screen-company" className="mx-auto max-w-[1000px] px-6 py-10">
        <div className="text-[10px] tracking-[0.16em] text-faint">COMPANY</div>
        <h1 className="mt-2 text-[22px] font-semibold tracking-tight">R Intelligent Data</h1>
        <p className="mt-2.5 max-w-[620px] text-[12px] leading-[1.8] text-muted-foreground">
            A layer between everything that emits a row and the handful of people who have to act
            on one. Founded on the observation that most teams do not have a data problem; they
            have a reading problem.
        </p>

        <div className="mt-8 grid grid-cols-3 gap-4">
            {Pillar("WHAT WE BELIEVE".to_string(), "A row that nobody will ever read is a cost with a timestamp on it. The useful question is never how much you kept, it is what you could open.".to_string())}
            {Pillar("HOW WE BUILD".to_string(), "Small, legible, and checkable. If a page claims a capability, the reader should be able to press it and watch it happen, here, now.".to_string())}
            {Pillar("WHAT WE REFUSE".to_string(), "A demonstration that does something other than what it says. Every page of this application that describes rather than performs says which one it is doing.".to_string())}
        </div>

        <div className="mt-6 grid grid-cols-[1.2fr_1fr] gap-4">
            <div className="rounded-[6px] border border-[#d98a4a] bg-[#140e08] p-6">
                <div className="text-[10px] tracking-[0.16em] text-warm">WHAT THIS WINDOW IS</div>
                <p className="mt-3 text-[13px] leading-[1.85]">
                    A showcase application for rustx, a Rust GUI framework. The marketing around it is
                    written to be read, but the only thing behind it is this process: tables in
                    memory, a file dropped on the window, a CSV written to disk. There is no
                    company, no collector fleet and no warehouse, and no page here pretends
                    otherwise.
                </p>
                <div className="mt-5 flex gap-2">
                    <button className="slab rounded-[5px] px-3.5 py-[7px] text-[11px]" onClick={move |_| rustx::router::navigate("/console".to_string())}>
                        SEE WHAT IS REAL
                    </button>
                    <button className="slab rounded-[5px] px-3.5 py-[7px] text-[11px]" onClick={move |_| rustx::router::navigate("/docs/architecture".to_string())}>
                        HOW IT IS PUT TOGETHER
                    </button>
                </div>
            </div>

            <div className="rounded-[6px] border border-border bg-card p-6">
                <div className="text-[10px] tracking-[0.16em] text-faint">THIS MACHINE</div>
                <div className="mt-3 flex flex-col gap-2">
                    {machine.iter().map(|fact| Fact(fact.0.clone(), fact.1.clone())).collect::<Vec<_>>()}
                    {Fact("tables held".to_string(), tables.len().to_string())}
                    {Fact("files taken in".to_string(), imported.to_string())}
                </div>
                <button className="slab mt-5 w-full rounded-[5px] px-3 py-[7px] text-[10px]" onClick={move |_| open_a_terminal()}>
                    OPEN A TERMINAL HERE
                </button>
            </div>
        </div>

        <p className="mt-5 text-[11px] text-muted-foreground">{status.clone()}</p>
    </div>;
}

fn Pillar(name: String, text: String) -> Element {
    return <div key={name.clone()} className="rounded-[6px] border border-border bg-card p-5">
        <div className="text-[9px] tracking-[0.16em] text-faint">{name}</div>
        <p className="mt-3 text-[12px] leading-[1.75] text-muted-foreground">{text}</p>
    </div>;
}

fn Fact(name: String, value: String) -> Element {
    return <div key={name.clone()} className="flex items-baseline gap-3 text-[11px]">
        <span className="shrink-0 text-faint">{name}</span>
        <span className="ml-auto min-w-0 truncate text-right">{value}</span>
    </div>;
}

fn read_machine() -> Vec<(String, String)> {
    let theme = match rustx::os::system::theme() {
        Ok(found) => format!("{found:?}").to_lowercase(),
        Err(_) => "unknown".to_string(),
    };
    let shell = match rustx::terminal::shell() {
        Ok(found) => found.name,
        Err(_) => "none found".to_string(),
    };
    vec![
        ("system theme".to_string(), theme),
        ("locale".to_string(), rustx::os::system::locale().unwrap_or_else(|_| "unknown".to_string())),
        ("timezone".to_string(), rustx::os::system::timezone().unwrap_or_else(|_| "unknown".to_string())),
        ("shell".to_string(), shell),
    ]
}

fn open_a_terminal() {
    let at = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    match rustx::terminal::open(&at) {
        Ok(()) => status = format!("a terminal is open in {}", at.display()),
        Err(error) => status = format!("no terminal: {error}"),
    }
}
