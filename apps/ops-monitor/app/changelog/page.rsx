// CHANGELOG — what changed in this console.
//
// The entries are the application's own copy and say so: there is no
// release feed behind a local demo. What is not copy is the filter and
// the copy-to-clipboard button, which put the selected release on the
// real clipboard.

/// One release.
#[derive(Clone)]
struct Release {
    version: String,
    date: String,
    kind: String,
    lines: Vec<String>,
}

let mut kind_filter: String = "ALL".to_string();
let mut said: String = String::new();

fn Page() -> Element {
    let shown: Vec<Release> = releases()
        .into_iter()
        .filter(|release| kind_filter == "ALL" || release.kind == kind_filter)
        .collect();

    return <div key="changelog" id="screen-changelog" className="pt-5">
        <div className="flex items-end">
            <div>
                <h1 className="text-[15px] font-semibold tracking-tight">Changelog</h1>
                <p className="mt-1 text-[11px] text-muted">
                    What this console gained, release by release. The list is the app's own copy.
                </p>
            </div>
            <div className="ml-auto flex rounded-[5px] border border-line bg-panel p-[3px]">
                {Filter("ALL".to_string())}
                {Filter("FEATURE".to_string())}
                {Filter("FIX".to_string())}
            </div>
        </div>

        <div id="releases" className="mt-5 flex flex-col">
            {shown.iter().map(|release| Entry(release.clone())).collect::<Vec<_>>()}
        </div>

        {if said.is_empty() {
            <span></span>
        } else {
            <p className="mt-4 text-[10px] text-faint">{said.clone()}</p>
        }}
    </div>;
}

fn Filter(name: String) -> Element {
    let active = kind_filter == name;
    let classes = format!(
        "rounded-[3px] px-3 py-[5px] text-[10px] tracking-[0.1em] {}",
        if active { "bg-[#2e2e2e] text-foreground" } else { "text-muted hover:text-foreground" }
    );
    let chosen = name.clone();
    return <button key={name.clone()} className={classes} onClick={move |_| kind_filter = chosen.clone()}>
        {name}
    </button>;
}

fn Entry(release: Release) -> Element {
    let tint = if release.kind == "FIX" { "#d9a13b" } else { "#5bbd7f" };
    let dot = format!("width:7px;height:7px;border-radius:50%;flex:none;background-color:{tint}");
    let to_copy = release.clone();

    return <div key={release.version.clone()} className="grid grid-cols-[150px_1fr] gap-6 border-b border-linesoft py-5 last:border-b-0">
        <div>
            <div className="flex items-center gap-2">
                <span style={dot}></span>
                <span className="text-[13px] font-semibold">{release.version.clone()}</span>
            </div>
            <div className="mt-1 pl-[15px] text-[10px] text-muted">{release.date.clone()}</div>
            <div className="mt-2 pl-[15px] text-[9px] tracking-[0.1em] text-faint">{release.kind.clone()}</div>
        </div>
        <div>
            <ul className="flex flex-col gap-1.5">
                {release.lines.iter().map(|line| Line(line.clone())).collect::<Vec<_>>()}
            </ul>
            <button
                className="mt-3 rounded-[5px] border border-line bg-panel2 px-2.5 py-[5px] text-[9px] tracking-[0.1em] text-muted hover:text-foreground"
                onClick={move |_| copy(to_copy.clone())}
            >
                COPY THESE NOTES
            </button>
        </div>
    </div>;
}

fn Line(text: String) -> Element {
    return <li key={text.clone()} className="flex gap-2 text-[12px] leading-[1.6]">
        <span className="text-faint">—</span>
        <span>{text}</span>
    </li>;
}

fn copy(release: Release) {
    let text = format!(
        "{} ({})\n{}",
        release.version,
        release.date,
        release.lines.iter().map(|line| format!("- {line}")).collect::<Vec<_>>().join("\n")
    );
    match rustx::clipboard::set_text(&text) {
        Ok(()) => said = format!("{} is on the clipboard.", release.version),
        Err(error) => said = format!("the clipboard refused: {error}"),
    }
}

fn releases() -> Vec<Release> {
    vec![
        Release {
            version: "2.4.0".to_string(),
            date: "this release".to_string(),
            kind: "FEATURE".to_string(),
            lines: vec![
                "Every tab of a service is a route of its own, so the address says which screen you are on.".to_string(),
                "Home, Services, Tasks, Docs, Status and Changelog are real pages, not placeholders.".to_string(),
                "Docs are addressable one topic at a time, under /docs/<topic>.".to_string(),
            ],
        },
        Release {
            version: "2.3.0".to_string(),
            date: "earlier".to_string(),
            kind: "FEATURE".to_string(),
            lines: vec![
                "The graphics adapter is read from the platform's own shell rather than guessed.".to_string(),
                "An armed rule raises a desktop notification once per crossing and badges the taskbar.".to_string(),
                "NEW DEPLOY runs a real script and reports the shell's exit code.".to_string(),
            ],
        },
        Release {
            version: "2.2.1".to_string(),
            date: "earlier".to_string(),
            kind: "FIX".to_string(),
            lines: vec![
                "The usage history survives a move between tabs instead of restarting.".to_string(),
                "A limit typed into a card is kept while the window stays open.".to_string(),
            ],
        },
        Release {
            version: "2.2.0".to_string(),
            date: "earlier".to_string(),
            kind: "FEATURE".to_string(),
            lines: vec![
                "Processor, memory and per-process load are read with sysinfo, once a second.".to_string(),
                "The network pill shows the machine's real link.".to_string(),
            ],
        },
        Release {
            version: "2.1.2".to_string(),
            date: "earlier".to_string(),
            kind: "FIX".to_string(),
            lines: vec![
                "The segmented usage bars no longer round a reading of 0% up to one lit tick.".to_string(),
            ],
        },
    ]
}
