// `/runs` — every run of every agent, newest first.
//
// Live: a loop files a new run every two seconds, and the filter boxes
// narrow the real list rather than a copy of it. Real: EXPORT writes a
// CSV through the system's save dialog and says so with a real
// notification, and clicking a row copies its id to the real clipboard.

#[derive(Clone)]
struct Entry {
    id: String,
    agent: String,
    intent: String,
    latency: f64,
    state: String,
    age: u64,
}

let mut runs_all: Vec<Entry> = Vec::new();
let mut runs_filter: String = String::new();
let mut runs_state: String = "all".to_string();
let mut runs_note: String = String::new();
let mut runs_tick: u64 = 0;

fn Page() -> Element {
    effect((), || {
        if runs_all.is_empty() {
            runs_all = (0..18).map(seeded).collect();
        }
    });

    async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            runs_tick += 1;
            let mut all = runs_all.clone();
            all.insert(0, seeded(100 + runs_tick as usize));
            all.truncate(40);
            runs_all = all;
        }
    };

    let needle = runs_filter.to_lowercase();
    let wanted = runs_state.clone();
    let shown: Vec<Entry> = runs_all
        .iter()
        .filter(|entry| wanted == "all" || entry.state == wanted)
        .filter(|entry| {
            needle.is_empty()
                || entry.agent.to_lowercase().contains(&needle)
                || entry.intent.to_lowercase().contains(&needle)
                || entry.id.contains(&needle)
        })
        .cloned()
        .collect();

    return <div key="runs" id="screen-runs" className="px-8 py-7">
        <div className="text-[13px] text-[#8b8f8b]">Workspace</div>
        <div className="mt-2 flex items-center gap-4">
            <h1 className="text-[30px] font-semibold tracking-tight">Runs</h1>
            <span className="rounded-full bg-[#1c1e1c] px-3 py-1 text-[13px] text-[#8b8f8b]">
                {format!("{} shown of {}", shown.len(), runs_all.len())}
            </span>
            <input
                id="filter"
                className="ml-auto w-[260px] rounded-xl border border-border bg-[#131413] px-3 py-2 text-[13px] outline-none placeholder:text-[#5d615d]"
                placeholder="Filter by agent, intent or id"
                value={runs_filter.clone()}
                onInput={move |event| runs_filter = event.value}
            />
            <span className="flex rounded-xl border border-border bg-[#131413] p-1">
                {StateButton("all".to_string())}
                {StateButton("completed".to_string())}
                {StateButton("escalated".to_string())}
            </span>
            <button
                className="rounded-xl border border-border bg-[#131413] px-3 py-2 text-[13px] text-[#9aa09a] hover:text-foreground"
                onClick={move |_| export_runs()}
            >
                Export
            </button>
        </div>
        <p className="mt-2 text-[12px] text-[#5d615d]">{runs_note.clone()}</p>

        <div className="mt-5 rounded-2xl border border-border bg-[#101110] p-5">
            <table id="runs" className="w-full text-left">
                <thead>
                    <tr className="text-[13px] text-[#8b8f8b]">
                        <th className="pb-2 font-normal">Run</th>
                        <th className="pb-2 font-normal">Agent</th>
                        <th className="pb-2 font-normal">Intent</th>
                        <th className="pb-2 text-right font-normal">Latency</th>
                        <th className="pb-2 text-right font-normal">Age</th>
                        <th className="pb-2 text-right font-normal">Status</th>
                    </tr>
                </thead>
                <tbody>
                    {shown.iter().map(|entry| Row(entry.clone())).collect::<Vec<_>>()}
                </tbody>
            </table>
        </div>
    </div>;
}

fn StateButton(name: String) -> Element {
    let here = runs_state == name;
    let classes = format!(
        "rounded-lg px-3 py-1.5 text-[13px] {}",
        if here { "bg-[#232523] text-foreground" } else { "text-[#8b8f8b] hover:text-foreground" }
    );
    let picked = name.clone();
    return <button key={name.clone()} className={classes} onClick={move |_| runs_state = picked.clone()}>
        {name}
    </button>;
}

fn Row(entry: Entry) -> Element {
    let pill = format!(
        "border-radius:999px;padding:2px 10px;font-size:12px;background-color:{};color:{}",
        if entry.state == "escalated" { "#2a2320" } else { "#133021" },
        if entry.state == "escalated" { "#d6a259" } else { "#3ddc84" }
    );
    let id = entry.id.clone();
    let slug = entry.agent.to_lowercase().replace(' ', "-");
    return <tr key={entry.id.clone()} className="border-t border-border text-[13px]">
        <td className="py-2.5 font-mono text-[#b6bbb6]">
            <button onClick={move |_| copy_run(id.clone())}>{entry.id.clone()}</button>
        </td>
        <td className="py-2.5">
            <button className="hover:text-[#3ddc84]" onClick={move |_| rustx::router::navigate(format!("/agent/{slug}"))}>
                {entry.agent.clone()}
            </button>
        </td>
        <td className="py-2.5 text-[#8b8f8b]">{entry.intent.clone()}</td>
        <td className="py-2.5 text-right font-mono text-[#b6bbb6]">{format!("{:.0}ms", entry.latency)}</td>
        <td className="py-2.5 text-right font-mono text-[#8b8f8b]">{format!("{}s", entry.age)}</td>
        <td className="py-2.5 text-right"><span style={pill}>{entry.state.clone()}</span></td>
    </tr>;
}

fn copy_run(id: String) {
    match rustx::clipboard::set_text(&id) {
        Ok(()) => runs_note = format!("{id} is on the clipboard"),
        Err(error) => runs_note = format!("the clipboard refused: {error}"),
    }
}

fn export_runs() {
    let ask = rustx::dialog::Ask::new("Export runs")
        .filter("Comma separated", ["csv"])
        .named("runs.csv");
    let Ok(path) = rustx::dialog::save(ask) else {
        runs_note = "the export was cancelled".to_string();
        return;
    };
    let mut text = String::from("id,agent,intent,latency_ms,state\n");
    for entry in runs_all.iter() {
        text.push_str(&format!(
            "{},{},{},{:.0},{}\n",
            entry.id, entry.agent, entry.intent, entry.latency, entry.state
        ));
    }
    match std::fs::write(&path, text) {
        Ok(()) => {
            runs_note = format!("written to {}", path.display());
            let _ = rustx::notify::show(rustx::notify::What::new(
                "Vantage",
                format!("{} runs exported.", runs_all.len()),
            ));
        }
        Err(error) => runs_note = format!("the export failed: {error}"),
    }
}

fn seeded(index: usize) -> Entry {
    let agents = ["Support Triage", "Refund Copilot", "Lead Qualifier", "Doc Indexer", "Churn Watcher"];
    let intents = ["refund status", "password reset", "shipping delay", "duplicate charge", "plan upgrade", "order cancelled"];
    let step = index as f64;
    Entry {
        id: format!("run_{:06x}", 0x4f1a20 + index * 0x1d7),
        agent: agents[index % agents.len()].to_string(),
        intent: intents[index % intents.len()].to_string(),
        latency: 320.0 + (step * 0.73).sin().abs() * 1600.0,
        state: if index % 7 == 3 { "escalated".to_string() } else { "completed".to_string() },
        age: (index as u64 * 17) % 900,
    }
}
