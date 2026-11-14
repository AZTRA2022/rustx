// `/` — the Overview the sidebar's first item points at.
//
// The agent screen is where the work is; this one is the fleet at a
// glance, plus the one card that is about the machine rather than the
// product: locale, timezone, theme and power, read from
// `rustx::os::system` so that at least one number on this screen is
// not the demo's own.

#[derive(Clone)]
struct Line {
    name: String,
    slug: String,
    runs: u64,
    rate: f64,
    live: bool,
}

let mut over_fleet: Vec<Line> = Vec::new();
let mut over_machine: Vec<(String, String)> = Vec::new();

fn Page() -> Element {
    effect((), || {
        if over_fleet.is_empty() {
            over_fleet = vec![
                Line { name: "Support Triage".to_string(), slug: "support-triage".to_string(), runs: 12_480, rate: 97.4, live: true },
                Line { name: "Refund Copilot".to_string(), slug: "refund-copilot".to_string(), runs: 3_902, rate: 99.1, live: false },
                Line { name: "Lead Qualifier".to_string(), slug: "lead-qualifier".to_string(), runs: 8_117, rate: 95.8, live: true },
                Line { name: "Doc Indexer".to_string(), slug: "doc-indexer".to_string(), runs: 640, rate: 100.0, live: false },
                Line { name: "Churn Watcher".to_string(), slug: "churn-watcher".to_string(), runs: 1_204, rate: 92.3, live: false },
            ];
        }
        over_machine = read_machine();
    });

    let runs: u64 = over_fleet.iter().map(|line| line.runs).sum();
    let live = over_fleet.iter().filter(|line| line.live).count();

    return <div key="overview" id="screen-overview" className="px-8 py-7">
        <div className="text-[13px] text-[#8b8f8b]">Workspace</div>
        <h1 className="mt-2 text-[30px] font-semibold tracking-tight">Overview</h1>

        <div className="mt-6 grid grid-cols-3 gap-4">
            {Tile("Runs this period".to_string(), thousands(runs))}
            {Tile("Agents deployed".to_string(), over_fleet.len().to_string())}
            {Tile("Live right now".to_string(), live.to_string())}
        </div>

        <div className="mt-4 grid grid-cols-[1.6fr_1fr] gap-4">
            <div className="rounded-2xl border border-border bg-[#101110] p-5">
                <h2 className="text-[19px] font-semibold tracking-tight">Agents</h2>
                <table id="fleet-table" className="mt-4 w-full text-left">
                    <thead>
                        <tr className="text-[13px] text-[#8b8f8b]">
                            <th className="pb-2 font-normal">Agent</th>
                            <th className="pb-2 text-right font-normal">Runs</th>
                            <th className="pb-2 text-right font-normal">Success</th>
                            <th className="pb-2 text-right font-normal">State</th>
                        </tr>
                    </thead>
                    <tbody>
                        {over_fleet.iter().map(|line| FleetLine(line.clone())).collect::<Vec<_>>()}
                    </tbody>
                </table>
            </div>

            <div className="rounded-2xl border border-border bg-[#101110] p-5">
                <h2 className="text-[19px] font-semibold tracking-tight">This machine</h2>
                <p className="mt-1 text-[12px] text-[#5d615d]">Read from the system, not from the demo.</p>
                <div className="mt-4 flex flex-col gap-3">
                    {over_machine.iter().map(|fact| Fact(fact.0.clone(), fact.1.clone())).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    </div>;
}

fn Tile(label: String, value: String) -> Element {
    return <div key={label.clone()} className="rounded-2xl border border-border bg-[#101110] p-5">
        <div className="text-[14px] text-[#b6bbb6]">{label}</div>
        <div className="mt-3 font-mono text-[30px] tracking-[0.02em]">{value}</div>
    </div>;
}

fn FleetLine(line: Line) -> Element {
    let route = format!("/agent/{}", line.slug);
    let pill = format!(
        "border-radius:999px;padding:2px 10px;font-size:12px;background-color:{};color:{}",
        if line.live { "#133021" } else { "#1c1e1c" },
        if line.live { "#3ddc84" } else { "#8b8f8b" }
    );
    return <tr key={line.slug.clone()} className="border-t border-border text-[13px]" onClick={move |_| rustx::router::navigate(route.clone())}>
        <td className="py-2.5">{line.name.clone()}</td>
        <td className="py-2.5 text-right font-mono text-[#b6bbb6]">{thousands(line.runs)}</td>
        <td className="py-2.5 text-right font-mono text-[#b6bbb6]">{format!("{:.1}%", line.rate)}</td>
        <td className="py-2.5 text-right">
            <span style={pill}>{if line.live { "live" } else { "idle" }}</span>
        </td>
    </tr>;
}

fn Fact(name: String, value: String) -> Element {
    return <div key={name.clone()} className="flex items-baseline">
        <span className="text-[13px] text-[#8b8f8b]">{name}</span>
        <span className="ml-auto font-mono text-[13px] text-[#b6bbb6]">{value}</span>
    </div>;
}

/// Four answers from the operating system, each one named even when the
/// platform has nothing to say.
fn read_machine() -> Vec<(String, String)> {
    let theme = match rustx::os::system::theme() {
        Ok(theme) => format!("{theme:?}").to_lowercase(),
        Err(_) => "unknown".to_string(),
    };
    let locale = rustx::os::system::locale().unwrap_or_else(|_| "unknown".to_string());
    let timezone = rustx::os::system::timezone().unwrap_or_else(|_| "unknown".to_string());
    let scale = match rustx::os::system::scale() {
        Ok(scale) => format!("{scale:.2}x"),
        Err(_) => "unknown".to_string(),
    };
    let power = match rustx::os::system::power() {
        Ok(power) => match power.level {
            Some(level) => format!("{level}%"),
            None => "on mains".to_string(),
        },
        Err(_) => "unknown".to_string(),
    };
    let screens = rustx::os::screen::screens().map(|all| all.len()).unwrap_or(0);

    vec![
        ("theme".to_string(), theme),
        ("locale".to_string(), locale),
        ("timezone".to_string(), timezone),
        ("display scale".to_string(), scale),
        ("screens".to_string(), screens.to_string()),
        ("power".to_string(), power),
    ]
}

fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}
