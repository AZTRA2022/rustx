// `/agent/<slug>` — the screen the whole app is about.
//
// The `[name]` segment is handed to `Page` as its argument, so a
// direct link, a reload, or a click in the sidebar all land on the
// same screen for the same name.
//
// What is live: a loop fills the current hour's bucket while the agent
// is running, the PAUSE button really stops it, and the 24h/7d/30d
// toggle really re-buckets the series rather than swapping a picture.
// "deployed N ago" counts from when this window opened, because that
// is the only deployment a local demo honestly has.
//
// What touches the machine: EXPORT writes a real CSV through the
// system's save dialog and says so with a real notification, and
// clicking a run copies its id to the real clipboard.

#[derive(rustx::ToValue, Clone)]
struct Bucket {
    at: String,
    completed: f64,
    escalated: f64,
}

#[derive(Clone)]
struct Run {
    id: String,
    intent: String,
    activity: String,
    latency: f64,
    state: String,
}

let mut run_series: Vec<Bucket> = Vec::new();
let mut run_recent: Vec<Run> = Vec::new();
let mut run_range: String = "24h".to_string();
let mut run_paused: bool = false;
let mut run_ticks: u64 = 0;
let mut run_total: u64 = 12_480;
let mut run_escalated: u64 = 324;
let mut run_note: String = String::new();
let mut run_opened_at: u64 = 0;

fn Page(name: String) -> Element {
    let title = titled(&name);

    effect((), || {
        if run_opened_at == 0 {
            run_opened_at = now();
        }
        if run_series.is_empty() {
            run_series = seed_series(&name);
            run_recent = seed_runs(&name);
        }
    });

    // The live half: one tick a second, into the last bucket, while the
    // agent is running. `run_paused` is read where it is declared, so
    // the PAUSE button changes what this loop does on its next pass.
    async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            if run_paused {
                continue;
            }
            run_ticks += 1;

            let step = run_ticks as f64;
            let completed = 6.0 + (step * 0.7).sin().abs() * 22.0;
            let escalated = (step * 0.31).cos().abs() * 2.0;

            let mut series = run_series.clone();
            if let Some(last) = series.last_mut() {
                last.completed += completed.round();
                last.escalated += escalated.round();
            }
            run_series = series;
            run_total += completed.round() as u64;
            run_escalated += escalated.round() as u64;

            if run_ticks.is_multiple_of(4) {
                let mut recent = run_recent.clone();
                recent.insert(0, fresh_run(run_ticks));
                recent.truncate(6);
                run_recent = recent;
            }
        }
    };

    return <div key="agent" id="screen-agent" className="px-8 py-7">
        {Header(title.clone())}
        {Kpis()}
        {RunsCard()}
        {RecentRuns()}
    </div>;
}

fn Header(title: String) -> Element {
    return <div className="flex items-start">
        <div>
            <div className="flex items-center gap-2 text-[13px] text-[#8b8f8b]">
                <span>Agents</span>
                <span className="text-[#4a4f4a]">/</span>
                <span>{format!("v14 · deployed {}", since(run_opened_at))}</span>
            </div>
            <div className="mt-2 flex items-center gap-3">
                <h1 id="agent-title" className="text-[30px] font-semibold tracking-tight">{title}</h1>
                {if run_paused {
                    <span className="rounded-full bg-[#2a2320] px-3 py-1 text-[13px] text-[#d6a259]">Paused</span>
                } else {
                    <span className="rounded-full bg-[#133021] px-3 py-1 text-[13px] text-[#3ddc84]">Live</span>
                }}
            </div>
        </div>
        <div className="ml-auto flex items-center gap-2">
            <button
                className="rounded-xl border border-border bg-[#131413] px-3 py-2 text-[12px] text-[#9aa09a] hover:text-foreground"
                onClick={move |_| export_csv()}
            >
                Export
            </button>
            <button
                id="pause"
                className="rounded-xl border border-border bg-[#131413] p-2.5 text-foreground hover:bg-[#1a1c1a]"
                onClick={move |_| run_paused = !run_paused}
            >
                {if run_paused {
                    <Play size={16} color="#e8eae8" />
                } else {
                    <Pause size={16} color="#e8eae8" />
                }}
            </button>
        </div>
    </div>;
}

fn Kpis() -> Element {
    let rate = if run_total == 0 { 0.0 } else { 100.0 - (run_escalated as f64 * 100.0 / run_total as f64) };
    let latency = 1.6 + (run_ticks as f64 * 0.21).sin().abs() * 0.5;

    return <div className="mt-6 grid grid-cols-3 gap-4">
        {Kpi(
            "Runs Today".to_string(),
            thousands(run_total),
            "+8.2%".to_string(),
            "#3ddc84".to_string(),
        )}
        {Kpi(
            "Success Rate".to_string(),
            format!("{:.1}%", rate),
            "+0.6%".to_string(),
            "#3ddc84".to_string(),
        )}
        {Kpi(
            "p95 latency".to_string(),
            format!("{:.2}s", latency),
            "−140ms".to_string(),
            "#3ddc84".to_string(),
        )}
    </div>;
}

fn Kpi(label: String, value: String, delta: String, colour: String) -> Element {
    let delta_colour = format!("color:{colour}");
    return <div key={label.clone()} className="rounded-2xl border border-border bg-[#101110] p-5">
        <div className="flex items-center">
            <span className="text-[14px] text-[#b6bbb6]">{label.clone()}</span>
            <span className="ml-auto"><Info size={15} color="#5d615d" /></span>
        </div>
        <div className="mt-3 font-mono text-[30px] tracking-[0.02em]">{value}</div>
        <div className="mt-3 flex items-end">
            <span className="flex items-center gap-1.5 text-[13px]" style={delta_colour}>
                <TrendingUp size={13} color={colour.clone()} />
                {delta}
            </span>
            <span className="ml-auto">{Spark(label.clone(), colour.clone())}</span>
        </div>
    </div>;
}

/// The little bar run beside each number. Fourteen ticks, the last one
/// lit — drawn as elements because a `style={{ … }}` object is read at
/// compile time and a height that comes from a number cannot go in one.
fn Spark(label: String, colour: String) -> Element {
    let seed: f64 = label.bytes().map(|byte| byte as f64).sum();
    return <span className="flex items-end gap-[3px]">
        {(0..14)
            .map(|index| {
                SparkBar(
                    format!("{label}-{index}"),
                    spark_style(index, seed, &colour, run_ticks),
                )
            })
            .collect::<Vec<_>>()}
    </span>;
}

fn SparkBar(name: String, paint: String) -> Element {
    return <span key={name} style={paint}></span>;
}

fn spark_style(index: usize, seed: f64, colour: &str, ticks: u64) -> String {
    let wave = ((index as f64 * 0.9 + seed + ticks as f64 * 0.05).sin().abs() * 16.0) + 6.0;
    let last = index == 13;
    format!(
        "display:inline-block;width:4px;border-radius:2px;height:{:.0}px;background-color:{}",
        if last { 26.0 } else { wave },
        if last { colour } else { "#2b2e2b" }
    )
}

fn RunsCard() -> Element {
    let shown = bucketed();
    return <div className="mt-4 rounded-2xl border border-border bg-[#101110] p-5">
        <div className="flex items-start">
            <div>
                <h2 className="text-[19px] font-semibold tracking-tight">Runs</h2>
                <p className="mt-0.5 text-[13px] text-[#8b8f8b]">{window_caption()}</p>
            </div>
            <div className="ml-auto flex items-center gap-5">
                <span className="flex items-center gap-2 text-[13px] text-[#b6bbb6]">
                    <span className="h-[9px] w-[9px] rounded-[2px] bg-[#6f746f]"></span>
                    Completed
                </span>
                <span className="flex items-center gap-2 text-[13px] text-[#b6bbb6]">
                    <span className="h-[9px] w-[9px] rounded-[2px] bg-[#3ddc84]"></span>
                    Escalated
                </span>
                <span className="flex rounded-xl border border-border bg-[#131413] p-1">
                    {RangeButton("24h".to_string())}
                    {RangeButton("7d".to_string())}
                    {RangeButton("30d".to_string())}
                </span>
            </div>
        </div>

        <div className="mt-5 rounded-xl bg-[#0c0d0c] p-4">
            <recharts::BarChart width={860} height={240} data={shown} barCategoryGap="22%">
                <recharts::CartesianGrid stroke="#1b1d1b" strokeDasharray="2 4" vertical={false} />
                <recharts::XAxis dataKey="at" stroke="#1b1d1b" tickLine={false} minTickGap={64} />
                <recharts::YAxis stroke="#1b1d1b" tickLine={false} width={44} />
                <recharts::Tooltip isAnimationActive={false} cursor={false} />
                <recharts::Bar dataKey="completed" name="Completed" stackId="runs" fill="url(#hatch)" isAnimationActive={false} />
                <recharts::Bar dataKey="escalated" name="Escalated" stackId="runs" fill="#3ddc84" isAnimationActive={false} />
            </recharts::BarChart>
        </div>
    </div>;
}

fn RangeButton(name: String) -> Element {
    let here = run_range == name;
    let classes = format!(
        "rounded-lg px-3 py-1.5 text-[13px] {}",
        if here { "bg-[#232523] text-foreground" } else { "text-[#8b8f8b] hover:text-foreground" }
    );
    let chosen = name.clone();
    return <button key={name.clone()} className={classes} onClick={move |_| run_range = chosen.clone()}>
        {name}
    </button>;
}

fn RecentRuns() -> Element {
    return <div className="mt-4 rounded-2xl border border-border bg-[#101110] p-5">
        <div className="flex items-center">
            <h2 className="text-[19px] font-semibold tracking-tight">Recent runs</h2>
            <span className="ml-auto text-[12px] text-[#5d615d]">{run_note.clone()}</span>
        </div>
        <table id="runs" className="mt-4 w-full text-left">
            <thead>
                <tr className="text-[13px] text-[#8b8f8b]">
                    <th className="pb-2 font-normal">Run</th>
                    <th className="pb-2 font-normal">Intent</th>
                    <th className="pb-2 font-normal">Activity</th>
                    <th className="pb-2 text-right font-normal">Latency</th>
                    <th className="pb-2 text-right font-normal">Status</th>
                </tr>
            </thead>
            <tbody>
                {run_recent.iter().map(|run| RunRow(run.clone())).collect::<Vec<_>>()}
            </tbody>
        </table>
    </div>;
}

fn RunRow(run: Run) -> Element {
    let pill = format!(
        "border-radius:999px;padding:2px 10px;font-size:12px;background-color:{};color:{}",
        if run.state == "escalated" { "#2a2320" } else { "#133021" },
        if run.state == "escalated" { "#d6a259" } else { "#3ddc84" }
    );
    let id = run.id.clone();
    return <tr key={run.id.clone()} className="border-t border-border text-[13px]" onClick={move |_| copy_run(id.clone())}>
        <td className="py-2.5 font-mono text-[#b6bbb6]">{run.id.clone()}</td>
        <td className="py-2.5">{run.intent.clone()}</td>
        <td className="py-2.5 text-[#8b8f8b]">{run.activity.clone()}</td>
        <td className="py-2.5 text-right font-mono text-[#b6bbb6]">{format!("{:.0}ms", run.latency)}</td>
        <td className="py-2.5 text-right"><span style={pill}>{run.state.clone()}</span></td>
    </tr>;
}

// ---- what the buttons do ---------------------------------------------

/// A real CSV, through the system's own save dialog, announced with a
/// real notification.
fn export_csv() {
    let ask = rustx::dialog::Ask::new("Export runs")
        .filter("Comma separated", ["csv"])
        .named("runs.csv");
    let Ok(path) = rustx::dialog::save(ask) else {
        run_note = "the export was cancelled".to_string();
        return;
    };

    let mut text = String::from("bucket,completed,escalated\n");
    for bucket in bucketed() {
        text.push_str(&format!("{},{},{}\n", bucket.at, bucket.completed, bucket.escalated));
    }

    match std::fs::write(&path, text) {
        Ok(()) => {
            run_note = format!("exported to {}", path.display());
            let _ = rustx::notify::show(rustx::notify::What::new(
                "Vantage",
                format!("{} buckets written to {}", bucketed().len(), path.display()),
            ));
        }
        Err(error) => run_note = format!("the export failed: {error}"),
    }
}

fn copy_run(id: String) {
    match rustx::clipboard::set_text(&id) {
        Ok(()) => run_note = format!("{id} is on the clipboard"),
        Err(error) => run_note = format!("the clipboard refused: {error}"),
    }
}

// ---- plain functions --------------------------------------------------

/// The series the chosen range asks for, built from the one real
/// series rather than from three pictures.
fn bucketed() -> Vec<Bucket> {
    let hourly = run_series.clone();
    if run_range == "24h" {
        return hourly;
    }
    let per = if run_range == "7d" { 7 } else { 30 };
    let group = (hourly.len() / per).max(1);
    let mut out: Vec<Bucket> = Vec::new();
    for (index, chunk) in hourly.chunks(group).enumerate() {
        out.push(Bucket {
            at: format!("D{}", index + 1),
            completed: chunk.iter().map(|bucket| bucket.completed).sum::<f64>(),
            escalated: chunk.iter().map(|bucket| bucket.escalated).sum::<f64>(),
        });
    }
    out
}

fn window_caption() -> String {
    if run_range == "24h" {
        return "Last 24 Hours · Hourly".to_string();
    }
    if run_range == "7d" {
        return "Last 7 Days · Daily".to_string();
    }
    "Last 30 Days · Daily".to_string()
}

fn seed_series(name: &str) -> Vec<Bucket> {
    let seed: f64 = name.bytes().map(|byte| byte as f64).sum();
    (0..24)
        .map(|hour| {
            let wave = ((hour as f64 * 0.42 + seed * 0.07).sin() + 1.0) / 2.0;
            Bucket {
                at: format!("{:02}:00", hour),
                completed: (180.0 + wave * 520.0).round(),
                escalated: (6.0 + wave * 46.0).round(),
            }
        })
        .collect()
}

fn seed_runs(name: &str) -> Vec<Run> {
    let intents = [
        ("refund status", "tool: orders.lookup"),
        ("password reset", "tool: identity.verify"),
        ("order cancelled", "escalated to human"),
        ("shipping delay", "tool: carrier.track"),
        ("duplicate charge", "tool: payments.search"),
        ("plan upgrade", "tool: billing.quote"),
    ];
    let seed = name.len() as f64;
    intents
        .iter()
        .enumerate()
        .map(|(index, (intent, activity))| Run {
            id: format!("run_{:06x}", 0x4f1a20 + index * 0x1d7 + name.len() * 0x33),
            intent: intent.to_string(),
            activity: activity.to_string(),
            latency: 420.0 + ((index as f64 + seed) * 1.7).sin().abs() * 1400.0,
            state: if index == 2 { "escalated".to_string() } else { "completed".to_string() },
        })
        .collect()
}

fn fresh_run(ticks: u64) -> Run {
    let intents = ["refund status", "password reset", "shipping delay", "plan upgrade"];
    let which = (ticks / 4) as usize % intents.len();
    Run {
        id: format!("run_{:06x}", 0x700000 + ticks * 0x3b),
        intent: intents[which].to_string(),
        activity: "tool: orders.lookup".to_string(),
        latency: 380.0 + (ticks as f64 * 0.9).sin().abs() * 1500.0,
        state: if ticks.is_multiple_of(20) { "escalated".to_string() } else { "completed".to_string() },
    }
}

/// "support-triage" is what the address says; "Support Triage" is what
/// a person reads.
fn titled(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut letters = word.chars();
            match letters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

fn since(then: u64) -> String {
    if then == 0 {
        return "just now".to_string();
    }
    let seconds = now().saturating_sub(then);
    if seconds < 60 {
        return format!("{seconds}s ago");
    }
    if seconds < 3600 {
        return format!("{}m ago", seconds / 60);
    }
    format!("{}h ago", seconds / 3600)
}
