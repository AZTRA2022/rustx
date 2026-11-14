// Ops Monitor — the service console of "FORGE ORG".
//
// Everything with a number on this screen is read from the machine the
// app is running on, not mocked:
//
//   CPU      `sysinfo` — global usage, core count, per-process load
//   MEMORY   `sysinfo` — used and total, refreshed every second
//   GPU      the platform shell, through `rustx::terminal::run` — the
//            adapter's own name and its live utilisation counter
//   NETWORK  `rustx::os::net` — the pill next to the service name says
//            what the machine's link really is
//
// And the buttons do what they say. LIVE LOGS opens a real terminal in
// the deploy directory (`rustx::terminal::open`). NEW DEPLOY runs a
// real script in the platform's shell and shows its exit code and its
// first line of output. CONFIGURE opens a real folder picker. An alert
// whose toggle is on raises a real desktop notification the moment its
// metric crosses, once per crossing, and flashes the taskbar.
//
// What is *not* real, and is named rather than hidden: the service
// identity (name, description, deploy history) is this app's own copy,
// because there is no control plane behind a local demo.

/// One second of the machine, as the chart reads it.
#[derive(rustx::ToValue, Clone)]
struct Sample {
    at: String,
    // Not `cpu` / `gpu`: a struct field whose name is also the name of a
    // value of this page is rewritten by the compiler into a read of
    // that value, which is not valid Rust in field position. Named
    // apart, on purpose.
    cpu_pct: f64,
    gpu_pct: f64,
    mem_pct: f64,
}

/// A rule the console watches, and whether it is armed.
#[derive(Clone)]
struct Alert {
    title: String,
    rule: String,
    metric: String,
    over: f64,
    critical: bool,
    on: bool,
}

// ---- what the machine says -------------------------------------------

let mut cpu: f64 = 0.0;
let mut cores: usize = 0;
let mut mem_used: u64 = 0;
let mut mem_total: u64 = 0;
let mut gpu: f64 = 0.0;
let mut gpu_name: String = String::new();
let mut gpu_vram: u64 = 0;
let mut gpu_reachable: bool = true;
let mut history: Vec<Sample> = Vec::new();
let mut processes: Vec<(String, f64, u64)> = Vec::new();
let mut samples: u64 = 0;
let mut link: String = "…".to_string();

// ---- what the console remembers --------------------------------------

let mut tab: String = "RESOURCE".to_string();
let mut window_name: String = "10 MIN".to_string();
let mut alerts: Vec<Alert> = Vec::new();
let mut raised: u32 = 0;
let mut cpu_limit: String = String::new();
let mut gpu_limit: String = "1".to_string();
let mut mem_limit: String = String::new();
let mut deploy_dir: String = String::new();
let mut deploy_line: String = "no deploy run from this window yet".to_string();
let mut shell_name: String = String::new();

fn Page() -> Element {
    // Built once, and from the machine: the alert list starts on the
    // three rules the console ships with, the limits start at what this
    // machine actually has, and the deploy directory is where the app
    // was launched from.
    effect((), || {
        if alerts.is_empty() {
            alerts = vec![
                Alert {
                    title: "GPU usage above 90%".to_string(),
                    rule: "Triggers when the graphics adapter reports over 90% utilisation".to_string(),
                    metric: "gpu".to_string(),
                    over: 90.0,
                    critical: false,
                    on: true,
                },
                Alert {
                    title: "CPU usage above 85%".to_string(),
                    rule: "Triggers when the processor stays over 85% on a sample".to_string(),
                    metric: "cpu".to_string(),
                    over: 85.0,
                    critical: true,
                    on: true,
                },
                Alert {
                    title: "Memory above 90%".to_string(),
                    rule: "Triggers when resident memory passes 90% of the machine".to_string(),
                    metric: "memory".to_string(),
                    over: 90.0,
                    critical: false,
                    on: false,
                },
            ];
        }
        if deploy_dir.is_empty() {
            deploy_dir = std::env::current_dir()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| ".".to_string());
        }
        shell_name = match rustx::terminal::shell() {
            Ok(shell) => shell.name,
            Err(_) => "none found".to_string(),
        };
        link = match rustx::os::net::network() {
            Ok(network) => format!("{:?}", network.link).to_uppercase(),
            Err(_) => "UNKNOWN".to_string(),
        };
    });

    // The machine, once a second. Reads `alerts` where it is declared —
    // so arming a rule from the screen changes what this loop watches
    // on its next pass, with nothing posted between them.
    async move {
        let mut system = sysinfo::System::new_all().unwrap();
        let mut over: [bool; 8] = [false; 8];

        loop {
            system.refresh_cpu_usage();
            system.refresh_memory();
            system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

            let usage = system.global_cpu_usage() as f64;
            let used = system.used_memory();
            let total = system.total_memory();
            let memory_percent = if total == 0 { 0.0 } else { used as f64 * 100.0 / total as f64 };

            let mut heaviest: Vec<(String, f64, u64)> = system
                .processes()
                .values()
                .map(|process| {
                    (
                        process.name().to_string_lossy().to_string(),
                        process.cpu_usage() as f64,
                        process.memory(),
                    )
                })
                .collect();
            heaviest.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            heaviest.truncate(8);

            // The limits start at what this machine actually has, the
            // first time the machine is read rather than before.
            if cpu_limit.is_empty() {
                cpu_limit = system.cpus().len().to_string();
            }
            if mem_limit.is_empty() && total > 0 {
                mem_limit = (total / 1_048_576).to_string();
            }

            cpu = usage;
            cores = system.cpus().len();
            mem_used = used;
            mem_total = total;
            processes = heaviest;
            samples += 1;

            let gpu_now = gpu;
            history.push(Sample {
                at: clock(),
                cpu_pct: (usage * 10.0).round() / 10.0,
                gpu_pct: (gpu_now * 10.0).round() / 10.0,
                mem_pct: (memory_percent * 10.0).round() / 10.0,
            });
            if history.len() > 480 {
                history.remove(0);
            }

            // One notification per crossing, not one per second.
            let armed = alerts.clone();
            for (index, alert) in armed.iter().enumerate() {
                if index >= over.len() {
                    break;
                }
                let value = match alert.metric.as_str() {
                    "cpu" => usage,
                    "gpu" => gpu_now,
                    _ => memory_percent,
                };
                let crossing = alert.on && value > alert.over;
                if crossing && !over[index] {
                    let _ = rustx::notify::show(rustx::notify::What::new(
                        "FORGE ORG · runner-gateway",
                        format!("{} — now at {:.0}%.", alert.title, value),
                    ));
                    let _ = rustx::os::attention::badge(Some(raised + 1));
                    raised += 1;
                }
                over[index] = crossing;
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    };

    // The graphics card, through the platform's own shell. Slower than
    // the rest and blocking while it runs, so it goes on a blocking
    // thread of the runtime and reports back here when it lands.
    async move {
        // The adapter is asked for once: its name does not change while
        // the window is open, and every call to the shell is a process
        // start with the user's profile behind it.
        if let Ok(Some(output)) =
            tokio::task::spawn_blocking(|| rustx::terminal::run(ADAPTER_SCRIPT, None).ok()).await
        {
            if output.worked() {
                let line = output.out.trim().to_string();
                if let Some((name, vram)) = line.split_once('|') {
                    gpu_name = name.trim().to_string();
                    gpu_vram = vram.trim().parse::<u64>().unwrap_or(0);
                } else if !line.is_empty() {
                    gpu_name = line;
                }
            }
        }

        loop {
            let answer =
                tokio::task::spawn_blocking(|| rustx::terminal::run(LOAD_SCRIPT, None).ok()).await;

            if let Ok(load) = answer {
                match load {
                    Some(output) if output.worked() && !output.out.trim().is_empty() => {
                        gpu = output.out.trim().parse::<f64>().unwrap_or(0.0).clamp(0.0, 100.0);
                        gpu_reachable = true;
                    }
                    _ => gpu_reachable = false,
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(8000)).await;
        }
    };

    return <div className="min-h-screen bg-background text-foreground">
        {TopBar()}
        <div className="mx-auto w-full max-w-[1240px] px-7 pb-10">
            {Breadcrumb()}
            {ServiceHead()}
            {Description()}
            {TabBar()}
            {Body()}
        </div>
    </div>;
}

// ---- chrome ----------------------------------------------------------

fn TopBar() -> Element {
    return <header className="flex h-11 items-center gap-6 border-b border-line bg-panel px-5">
        <div className="flex items-center gap-2">
            <span className="flex h-[18px] w-[18px] items-center justify-center rounded-[4px] bg-brand text-[10px] font-bold text-black">F</span>
            <span className="text-[12px] font-semibold tracking-[0.14em]">FORGE ORG</span>
            <span className="rounded-[3px] bg-[#2f2f2f] px-1.5 py-[1px] text-[9px] tracking-[0.12em] text-muted">PRO</span>
            <ChevronDown size={11} color="#6a6a66" />
        </div>
        <nav className="ml-auto flex items-center gap-1">
            {NavItem("Home".to_string(), false)}
            {NavItem("Services".to_string(), true)}
            {NavItem("Tasks".to_string(), false)}
            {NavItem("Docs".to_string(), false)}
            {NavItem("Status".to_string(), false)}
            {NavItem("Changelog".to_string(), false)}
        </nav>
        <div className="flex items-center gap-3 pl-3">
            <button
                className="relative text-muted hover:text-foreground"
                onClick={move |_| { let _ = rustx::os::attention::badge(None); raised = 0; }}
            >
                <Bell size={14} color="#8a8a85" />
                {if raised > 0 {
                    <span className="absolute -right-[3px] -top-[3px] h-[6px] w-[6px] rounded-full bg-brand"></span>
                } else {
                    <span></span>
                }}
            </button>
            <div className="flex items-center gap-2">
                <span className="flex h-[18px] w-[18px] items-center justify-center rounded-[4px] bg-[#8b3f2a] text-[9px] font-bold">M</span>
                <span className="text-[11px] text-muted">Mira Okafor</span>
                <ChevronDown size={11} color="#6a6a66" />
            </div>
        </div>
    </header>;
}

fn NavItem(label: String, here: bool) -> Element {
    let classes = format!(
        "rounded-[4px] px-2.5 py-1 text-[11px] {}",
        if here { "bg-[#2a2a2a] text-foreground" } else { "text-muted hover:text-foreground" }
    );
    return <span key={label.clone()} className={classes}>{label}</span>;
}

fn Breadcrumb() -> Element {
    return <div className="flex items-center gap-2 py-3 text-[11px] text-muted">
        <LayoutGrid size={11} color="#6a6a66" />
        <span>Services</span>
        <span className="text-faint">/</span>
        <span className="text-foreground">Service Details</span>
    </div>;
}

fn ServiceHead() -> Element {
    return <div className="flex items-start gap-3">
        <span className="mt-[2px] flex h-8 w-8 items-center justify-center rounded-[6px] bg-brand">
            <Cog size={17} color="#140b06" />
        </span>
        <div>
            <h1 className="text-[15px] font-semibold tracking-tight">runner-gateway</h1>
            <div className="mt-1.5 flex items-center gap-1.5">
                <span className="rounded-[3px] bg-[#16331f] px-1.5 py-[2px] text-[9px] tracking-[0.1em] text-[#5bbd7f]">ACTIVE</span>
                {Chip("API".to_string())}
                {Chip("EDGE".to_string())}
                {Chip("AGENT RUNTIME".to_string())}
                {Chip(link.clone())}
            </div>
        </div>
        <div className="ml-auto flex items-center gap-2">
            <button
                className="flex items-center gap-1.5 rounded-[5px] border border-line bg-panel2 px-3 py-[7px] text-[10px] tracking-[0.1em] text-foreground hover:border-faint"
                onClick={move |_| open_a_terminal()}
            >
                <SquareArrowOutUpRight size={11} color="#d6d6d4" />
                LIVE LOGS
            </button>
            <button
                className="flex items-center gap-1.5 rounded-[5px] border border-line bg-[#262626] px-3 py-[7px] text-[10px] tracking-[0.1em] text-foreground hover:border-faint"
                onClick={move |_| deploy()}
            >
                <Plus size={11} color="#d6d6d4" />
                NEW DEPLOY
            </button>
        </div>
    </div>;
}

fn Chip(label: String) -> Element {
    return <span key={label.clone()} className="rounded-[3px] border border-line px-1.5 py-[2px] text-[9px] tracking-[0.1em] text-muted">
        {label}
    </span>;
}

fn Description() -> Element {
    return <div className="mt-4">
        <div className="text-[10px] tracking-[0.12em] text-muted">Description</div>
        <p className="mt-1.5 max-w-[560px] text-[12px] leading-[1.65] text-foreground">
            Edge gateway that queues agent tool calls, streams tokens back
            to clients and retries failed runs across three regions.
        </p>
        <p id="deploy-line" className="mt-1.5 text-[11px] text-faint">{deploy_line.clone()}</p>
    </div>;
}

fn TabBar() -> Element {
    return <div className="mt-4 flex w-fit rounded-[5px] border border-line bg-panel p-[3px]">
        {Tab("OVERVIEW".to_string())}
        {Tab("RESOURCE".to_string())}
        {Tab("ANALYTICS".to_string())}
        {Tab("SETTINGS".to_string())}
    </div>;
}

fn Tab(name: String) -> Element {
    let here = tab == name;
    let classes = format!(
        "rounded-[3px] px-3 py-[6px] text-[10px] tracking-[0.11em] {}",
        if here { "bg-[#2e2e2e] text-foreground" } else { "text-muted hover:text-foreground" }
    );
    let chosen = name.clone();
    return <button key={name.clone()} className={classes} onClick={move |_| tab = chosen.clone()}>
        {name}
    </button>;
}

fn Body() -> Element {
    return <div id="body">
        {if tab == "OVERVIEW" {
            Overview()
        } else if tab == "ANALYTICS" {
            Analytics()
        } else if tab == "SETTINGS" {
            Settings()
        } else {
            Resource()
        }}
    </div>;
}

// ---- RESOURCE --------------------------------------------------------

fn Resource() -> Element {
    return <>
        <div className="mt-5 flex items-center">
            <h2 className="text-[10px] tracking-[0.14em] text-muted">RESOURCE MANAGEMENT</h2>
            <button
                className="ml-auto flex items-center gap-1.5 rounded-[5px] border border-line bg-panel2 px-3 py-[6px] text-[10px] tracking-[0.1em] text-muted hover:text-foreground"
                onClick={move |_| choose_deploy_dir()}
            >
                <Settings2 size={11} color="#8a8a85" />
                CONFIGURE
            </button>
        </div>

        <div className="mt-3 grid grid-cols-3 rounded-[7px] border border-line bg-panel">
            {CpuCard()}
            {GpuCard()}
            {MemoryCard()}
        </div>

        <div className="mt-4 grid grid-cols-[1fr_1fr] gap-4">
            {UsageHistory()}
            {AlertList()}
        </div>
    </>;
}

fn CpuCard() -> Element {
    let used_cores = cores as f64 * cpu / 100.0;
    return ResourceCard(
        "CPU".to_string(),
        "#4a86d6".to_string(),
        cpu,
        format!("{} CORES", cores),
        format!("{:.1} CORES", used_cores),
        "Cpu limit".to_string(),
        cpu_limit.clone(),
        "CORES".to_string(),
        0,
    );
}

fn GpuCard() -> Element {
    let shown = if gpu_reachable { gpu } else { 0.0 };
    let used = if gpu_reachable { format!("{:.1} GPUS", shown / 100.0) } else { "unreadable".to_string() };
    let vram = if gpu_vram > 0 { format!("{} MB VRAM", gpu_vram / 1_048_576) } else { "1x ADAPTER".to_string() };
    return ResourceCard(
        "GPU".to_string(),
        "#4ea972".to_string(),
        shown,
        vram,
        used,
        "Gpu limit".to_string(),
        gpu_limit.clone(),
        "GPUS".to_string(),
        1,
    );
}

fn MemoryCard() -> Element {
    let percent = if mem_total == 0 { 0.0 } else { mem_used as f64 * 100.0 / mem_total as f64 };
    return ResourceCard(
        "MEMORY".to_string(),
        "#d9793b".to_string(),
        percent,
        format!("{:.0} GB", mem_total as f64 / 1_073_741_824.0),
        format!("{:.1} GB", mem_used as f64 / 1_073_741_824.0),
        "Mem limit".to_string(),
        mem_limit.clone(),
        "MB".to_string(),
        2,
    );
}

/// One of the three cards: the three numbers, the segmented usage bar
/// and the editable limit at the foot.
fn ResourceCard(
    name: String,
    colour: String,
    percent: f64,
    allocated: String,
    used: String,
    limit_label: String,
    limit: String,
    limit_unit: String,
    which: usize,
) -> Element {
    let swatch = format!("height:9px;width:9px;border-radius:2px;background-color:{colour}");
    let used_colour = format!("color:{colour}");
    let badge = format!(
        "border-radius:3px;padding:1px 6px;font-size:9px;letter-spacing:.08em;color:{colour};background-color:{}1f",
        colour
    );
    let divider = if which == 2 { "" } else { "border-r border-line" };
    let classes = format!("px-5 py-3.5 {divider}");

    return <div key={name.clone()} className={classes}>
        <div className="flex items-center gap-2">
            <span style={swatch}></span>
            <span className="text-[11px] tracking-[0.12em]">{name.clone()}</span>
            <span className="ml-auto" style={badge}>{format!("{:.0}%", percent)}</span>
        </div>

        <div className="mt-3.5 grid grid-cols-3 gap-2">
            <div>
                <div className="text-[9px] tracking-[0.1em] text-faint">Allocated</div>
                <div className="mt-1 text-[12px]">{allocated}</div>
            </div>
            <div>
                <div className="text-[9px] tracking-[0.1em] text-faint">Used</div>
                <div className="mt-1 text-[12px]" style={used_colour}>{used}</div>
            </div>
            <div>
                <div className="text-[9px] tracking-[0.1em] text-faint">Utilization</div>
                <div className="mt-1 text-[12px]">{format!("{:.0}%", percent)}</div>
            </div>
        </div>

        <div className="mt-3.5 flex items-center justify-between text-[9px] tracking-[0.1em] text-faint">
            <span>Usage</span>
            <span>{format!("{:.0}%", percent)}</span>
        </div>
        {SegmentBar(name.clone(), percent, colour.clone())}

        <div className="mt-3 flex items-center justify-between">
            <span className="text-[10px] text-muted">{limit_label}</span>
            <span className="flex items-center gap-1 rounded-[4px] border border-line bg-[#131313] px-2 py-[3px]">
                <input
                    className="w-14 bg-transparent text-right text-[10px] text-foreground outline-none"
                    value={limit}
                    onInput={move |event| set_limit(which, event.value)}
                />
                <span className="rounded-[2px] bg-[#2a2a2a] px-1 text-[8px] tracking-[0.08em] text-faint">{limit_unit}</span>
            </span>
        </div>
    </div>;
}

/// The segmented bar of the mock: forty-four ticks, lit up to the
/// percentage. A `style={{ … }}` object is read at compile time and
/// takes literal strings only, so each tick's colour is built as a
/// declaration string instead.
fn SegmentBar(name: String, percent: f64, colour: String) -> Element {
    let total = 44usize;
    let lit = ((percent.clamp(0.0, 100.0) / 100.0) * total as f64).round() as usize;
    let on = format!("height:14px;flex:1;border-radius:1px;background-color:{colour}");
    let off = "height:14px;flex:1;border-radius:1px;background-color:#2a2a2a".to_string();

    return <div className="mt-1.5 flex gap-[2px]">
        {(0..total)
            .map(|index| Segment(
                format!("{name}-{index}"),
                if index < lit { on.clone() } else { off.clone() },
            ))
            .collect::<Vec<_>>()}
    </div>;
}

fn Segment(name: String, paint: String) -> Element {
    return <span key={name} style={paint}></span>;
}

fn UsageHistory() -> Element {
    let shown = visible_history();
    return <div className="rounded-[7px] border border-line bg-panel p-4">
        <div className="flex items-center">
            <h3 className="text-[10px] tracking-[0.14em] text-muted">USAGE HISTORY</h3>
            <button
                className="ml-auto flex items-center gap-2 rounded-[5px] border border-line bg-panel2 px-2.5 py-[5px] text-[10px] text-foreground"
                onClick={move |_| window_name = next_window(&window_name)}
            >
                {window_name.clone()}
                <ChevronDown size={10} color="#6a6a66" />
            </button>
        </div>

        <div className="mt-3 flex items-center gap-4">
            {Legend("CPU".to_string(), "#4a86d6".to_string())}
            {Legend("GPU".to_string(), "#4ea972".to_string())}
            {Legend("MEMORY".to_string(), "#d9793b".to_string())}
        </div>

        <div className="mt-2">
            <recharts::LineChart width={520} height={168} data={shown}>
                <recharts::CartesianGrid stroke="#242424" vertical={false} />
                <recharts::XAxis dataKey="at" stroke="#2b2b2b" tickLine={false} minTickGap={48} />
                <recharts::YAxis stroke="#2b2b2b" tickLine={false} domain={vec![0.0, 100.0]} width={34} unit="%" />
                <recharts::Tooltip isAnimationActive={false} cursor={false} />
                <recharts::Line type="monotone" dataKey="cpu_pct" name="CPU" stroke="#4a86d6" strokeWidth={1.5} dot={false} isAnimationActive={false} />
                <recharts::Line type="monotone" dataKey="gpu_pct" name="GPU" stroke="#4ea972" strokeWidth={1.5} dot={false} isAnimationActive={false} />
                <recharts::Line type="monotone" dataKey="mem_pct" name="MEMORY" stroke="#d9793b" strokeWidth={1.5} dot={false} isAnimationActive={false} />
            </recharts::LineChart>
        </div>
        <div className="mt-1 text-right text-[9px] tracking-[0.1em] text-faint">
            {format!("{} SAMPLES · NOW", samples)}
        </div>
    </div>;
}

fn Legend(name: String, colour: String) -> Element {
    let swatch = format!("height:7px;width:7px;border-radius:1px;background-color:{colour}");
    return <span key={name.clone()} className="flex items-center gap-1.5 text-[9px] tracking-[0.1em] text-muted">
        <span style={swatch}></span>
        {name}
    </span>;
}

fn AlertList() -> Element {
    return <div className="rounded-[7px] border border-line bg-panel p-4">
        <div className="flex items-center">
            <h3 className="text-[10px] tracking-[0.14em] text-muted">RESOURCE ALERT</h3>
            <button
                className="ml-auto flex items-center gap-1.5 rounded-[5px] border border-line bg-panel2 px-2.5 py-[5px] text-[10px] tracking-[0.1em] text-muted hover:text-foreground"
                onClick={move |_| add_alert()}
            >
                <Plus size={10} color="#8a8a85" />
                ADD ALERT
            </button>
        </div>
        <div id="alerts" className="mt-2 divide-y divide-[#242424]">
            {alerts.iter().enumerate().map(|(index, alert)| AlertRow(index, alert.clone())).collect::<Vec<_>>()}
        </div>
        <p className="mt-3 text-[10px] text-faint">
            {format!("{} notification(s) raised since this window opened.", raised)}
        </p>
    </div>;
}

fn AlertRow(index: usize, alert: Alert) -> Element {
    let icon_wrap = if alert.critical {
        "flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-[#3a1511]"
    } else {
        "flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-[#33280f]"
    };
    let track = format!(
        "width:30px;height:15px;border-radius:9px;display:inline-flex;align-items:center;padding:2px;background-color:{}",
        if alert.on { "#4a4a4a" } else { "#262626" }
    );
    let knob = format!(
        "width:11px;height:11px;border-radius:50%;background-color:#e6e6e4;transform:translateX({})",
        if alert.on { "15px" } else { "0px" }
    );

    return <div key={index.to_string()} className="flex items-start gap-3 py-3">
        <span className={icon_wrap}>
            {if alert.critical {
                <CircleAlert size={13} color="#e0533c" />
            } else {
                <TriangleAlert size={13} color="#d9a13b" />
            }}
        </span>
        <div className="min-w-0">
            <div className="text-[12px] text-[#9fb8dc]">{alert.title.clone()}</div>
            <div className="mt-0.5 text-[10px] leading-snug text-faint">{alert.rule.clone()}</div>
        </div>
        <button className="ml-auto mt-1 shrink-0" style={track} onClick={move |_| toggle_alert(index)}>
            <span style={knob}></span>
        </button>
    </div>;
}

// ---- OVERVIEW / ANALYTICS / SETTINGS ---------------------------------

fn Overview() -> Element {
    let memory_percent = if mem_total == 0 { 0.0 } else { mem_used as f64 * 100.0 / mem_total as f64 };
    return <div className="mt-6 grid grid-cols-[1fr_1fr] gap-4">
        <div className="rounded-[7px] border border-line bg-panel p-4">
            <h3 className="text-[10px] tracking-[0.14em] text-muted">SERVICE</h3>
            <div className="mt-3 grid grid-cols-2 gap-y-3">
                {Fact("Region".to_string(), "edge · 3 zones".to_string())}
                {Fact("Runtime".to_string(), "agent runtime".to_string())}
                {Fact("Link".to_string(), link.clone())}
                {Fact("Shell".to_string(), shell_name.clone())}
                {Fact("Processor".to_string(), format!("{} cores · {:.0}%", cores, cpu))}
                {Fact("Memory".to_string(), format!("{:.0}% of {:.0} GB", memory_percent, mem_total as f64 / 1_073_741_824.0))}
                {Fact("Adapter".to_string(), if gpu_name.is_empty() { "reading…".to_string() } else { gpu_name.clone() })}
                {Fact("Samples".to_string(), samples.to_string())}
            </div>
        </div>
        <div className="rounded-[7px] border border-line bg-panel p-4">
            <h3 className="text-[10px] tracking-[0.14em] text-muted">HEAVIEST PROCESSES</h3>
            <table id="processes" className="mt-3 w-full text-[11px]">
                <tbody>
                    {processes.iter().map(|row| ProcessRow(row.0.clone(), row.1, row.2)).collect::<Vec<_>>()}
                </tbody>
            </table>
        </div>
    </div>;
}

fn Fact(name: String, value: String) -> Element {
    return <div key={name.clone()}>
        <div className="text-[9px] tracking-[0.1em] text-faint">{name}</div>
        <div className="mt-1 truncate text-[11px]">{value}</div>
    </div>;
}

fn ProcessRow(name: String, load: f64, memory: u64) -> Element {
    return <tr key={name.clone()} className="border-b border-linesoft">
        <td className="truncate py-1.5 pr-2">{name}</td>
        <td className="py-1.5 text-right text-[#9fb8dc]">{format!("{:.1}%", load)}</td>
        <td className="py-1.5 pl-3 text-right text-muted">{format!("{:.2} GB", memory as f64 / 1_073_741_824.0)}</td>
    </tr>;
}

fn Analytics() -> Element {
    let shown = visible_history();
    return <div className="mt-6 rounded-[7px] border border-line bg-panel p-4">
        <div className="flex items-center">
            <h3 className="text-[10px] tracking-[0.14em] text-muted">USAGE HISTORY — FULL WIDTH</h3>
            <button
                className="ml-auto rounded-[5px] border border-line bg-panel2 px-2.5 py-[5px] text-[10px]"
                onClick={move |_| window_name = next_window(&window_name)}
            >
                {window_name.clone()}
            </button>
        </div>
        <div className="mt-3 flex items-center gap-4">
            {Legend("CPU".to_string(), "#4a86d6".to_string())}
            {Legend("GPU".to_string(), "#4ea972".to_string())}
            {Legend("MEMORY".to_string(), "#d9793b".to_string())}
        </div>
        <div className="mt-2">
            <recharts::LineChart width={1120} height={320} data={shown}>
                <recharts::CartesianGrid stroke="#242424" vertical={false} />
                <recharts::XAxis dataKey="at" stroke="#2b2b2b" tickLine={false} minTickGap={60} />
                <recharts::YAxis stroke="#2b2b2b" tickLine={false} domain={vec![0.0, 100.0]} width={34} unit="%" />
                <recharts::Tooltip isAnimationActive={false} cursor={false} />
                <recharts::Line type="monotone" dataKey="cpu_pct" name="CPU" stroke="#4a86d6" strokeWidth={1.5} dot={false} isAnimationActive={false} />
                <recharts::Line type="monotone" dataKey="gpu_pct" name="GPU" stroke="#4ea972" strokeWidth={1.5} dot={false} isAnimationActive={false} />
                <recharts::Line type="monotone" dataKey="mem_pct" name="MEMORY" stroke="#d9793b" strokeWidth={1.5} dot={false} isAnimationActive={false} />
            </recharts::LineChart>
        </div>
    </div>;
}

fn Settings() -> Element {
    return <div className="mt-6 max-w-[620px] rounded-[7px] border border-line bg-panel p-5">
        <h3 className="text-[10px] tracking-[0.14em] text-muted">SETTINGS</h3>

        <div className="mt-4 text-[9px] tracking-[0.1em] text-faint">Deploy directory</div>
        <div className="mt-1.5 flex items-center gap-2">
            <span id="deploy-dir" className="flex-1 truncate rounded-[4px] border border-line bg-[#131313] px-2.5 py-[6px] text-[11px]">
                {deploy_dir.clone()}
            </span>
            <button className="rounded-[5px] border border-line bg-panel2 px-3 py-[6px] text-[10px] tracking-[0.1em]" onClick={move |_| choose_deploy_dir()}>
                BROWSE
            </button>
            <button className="rounded-[5px] border border-line bg-panel2 px-3 py-[6px] text-[10px] tracking-[0.1em]" onClick={move |_| open_a_terminal()}>
                TERMINAL
            </button>
        </div>

        <div className="mt-5 text-[9px] tracking-[0.1em] text-faint">Machine report</div>
        <p className="mt-1.5 text-[11px] leading-relaxed text-muted">{machine_report()}</p>
        <button
            className="mt-2 rounded-[5px] border border-line bg-panel2 px-3 py-[6px] text-[10px] tracking-[0.1em]"
            onClick={move |_| {
                let _ = rustx::clipboard::set_text(&machine_report());
                let _ = rustx::notify::show(rustx::notify::What::new(
                    "FORGE ORG",
                    "The machine report is on the clipboard.",
                ));
            }}
        >
            COPY REPORT
        </button>
    </div>;
}

// ---- what the buttons do ---------------------------------------------

fn set_limit(which: usize, value: String) {
    if which == 0 {
        cpu_limit = value;
    } else if which == 1 {
        gpu_limit = value;
    } else {
        mem_limit = value;
    }
}

fn toggle_alert(index: usize) {
    let mut armed = alerts.clone();
    if let Some(alert) = armed.get_mut(index) {
        alert.on = !alert.on;
    }
    alerts = armed;
}

/// A rule built out of what the machine is doing right now: ten points
/// above the current reading, so it is a line that can actually be
/// crossed rather than a decoration.
fn add_alert() {
    let mut armed = alerts.clone();
    let over = (cpu + 10.0).clamp(5.0, 99.0);
    armed.push(Alert {
        title: format!("CPU usage above {:.0}%", over),
        rule: format!("Added at {} — the reading was {:.0}% then", clock(), cpu),
        metric: "cpu".to_string(),
        over,
        critical: false,
        on: true,
    });
    alerts = armed;
}

/// A real terminal, in the deploy directory, left to the user.
fn open_a_terminal() {
    let at = std::path::PathBuf::from(deploy_dir.clone());
    match rustx::terminal::open(&at) {
        Ok(()) => deploy_line = format!("a terminal is open in {}", at.display()),
        Err(error) => deploy_line = format!("no terminal: {error}"),
    }
}

/// A real script, in the platform's own shell, waited for.
fn deploy() {
    let at = std::path::PathBuf::from(deploy_dir.clone());
    let where_at = if at.is_dir() { Some(at.as_path()) } else { None };
    match rustx::terminal::run(DEPLOY_SCRIPT, where_at) {
        Ok(output) => {
            let first = output.out.lines().next().unwrap_or("").trim().to_string();
            deploy_line = format!("deploy exited {} — {}", output.code, first);
            let _ = rustx::notify::show(rustx::notify::What::new(
                "FORGE ORG · runner-gateway",
                format!("Deploy finished with code {}.", output.code),
            ));
        }
        Err(error) => deploy_line = format!("deploy refused: {error}"),
    }
}

fn choose_deploy_dir() {
    match rustx::dialog::folder(rustx::dialog::Ask::new("Deploy directory")) {
        Ok(path) => {
            deploy_dir = path.display().to_string();
            deploy_line = format!("deploy directory is now {}", path.display());
        }
        Err(_) => {
            deploy_line = "the folder picker was closed".to_string();
        }
    }
}

// ---- plain functions --------------------------------------------------

/// The slice of the real history the chosen window shows.
fn visible_history() -> Vec<Sample> {
    let keep = match window_name.as_str() {
        "2 MIN" => 120usize,
        "1 HOUR" => 480usize,
        _ => 300usize,
    };
    let all = history.clone();
    if all.len() <= keep {
        return all;
    }
    all[all.len() - keep..].to_vec()
}

fn next_window(current: &str) -> String {
    match current {
        "2 MIN" => "10 MIN".to_string(),
        "10 MIN" => "1 HOUR".to_string(),
        _ => "2 MIN".to_string(),
    }
}

fn machine_report() -> String {
    format!(
        "runner-gateway · {} cores at {:.0}% · {:.1} of {:.1} GB resident · adapter {} at {:.0}% · link {} · {} samples",
        cores,
        cpu,
        mem_used as f64 / 1_073_741_824.0,
        mem_total as f64 / 1_073_741_824.0,
        if gpu_name.is_empty() { "unknown".to_string() } else { gpu_name.clone() },
        gpu,
        link,
        samples
    )
}

/// The wall clock, as the chart's x axis reads it.
fn clock() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    let day = seconds % 86_400;
    format!("{:02}:{:02}:{:02}", day / 3600, (day % 3600) / 60, day % 60)
}

/// What the shell is asked for the graphics card. PowerShell on
/// Windows, and a best effort elsewhere — `rustx::terminal` reports
/// which shell it found, and a script that means nothing to it comes
/// back with a non-zero code, which is read as "unreadable" rather
/// than as a number.
#[cfg(windows)]
const ADAPTER_SCRIPT: &str = "$a = Get-CimInstance Win32_VideoController | Select-Object -First 1; \"$($a.Name)|$($a.AdapterRAM)\"";
#[cfg(not(windows))]
const ADAPTER_SCRIPT: &str = "lspci 2>/dev/null | grep -i -m1 'vga\\|3d' | cut -d: -f3";

#[cfg(windows)]
const LOAD_SCRIPT: &str = "$s=(Get-Counter '\\GPU Engine(*)\\Utilization Percentage' -ErrorAction Stop).CounterSamples | Measure-Object -Property CookedValue -Sum; [math]::Round($s.Sum,1)";
#[cfg(not(windows))]
const LOAD_SCRIPT: &str = "exit 1";

/// The deploy. A real script, run in the real shell: it prints where it
/// ran and what the machine calls itself, and its exit code is the one
/// the console shows.
#[cfg(windows)]
const DEPLOY_SCRIPT: &str = "\"deployed from $(Get-Location) on $env:COMPUTERNAME at $(Get-Date -Format HH:mm:ss)\"";
#[cfg(not(windows))]
const DEPLOY_SCRIPT: &str = "echo \"deployed from $(pwd) on $(hostname) at $(date +%H:%M:%S)\"";
