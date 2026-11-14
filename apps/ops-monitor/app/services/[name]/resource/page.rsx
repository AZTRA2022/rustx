// RESOURCE — the screen the mock is of.
//
// Three cards, a segmented usage bar on each, the usage history and
// the alert list. Everything with a number on it is the machine's:
// the processor and its cores and the memory from `sysinfo`, the
// adapter and its utilisation from the platform's shell, both read by
// the loops in the application's root layout and redeclared here by
// name.

let mut cpu: f64 = 0.0;
let mut cores: usize = 0;
let mut mem_used: u64 = 0;
let mut mem_total: u64 = 0;
let mut gpu: f64 = 0.0;
let mut gpu_vram: u64 = 0;
let mut gpu_reachable: bool = true;
let mut history: Vec<Sample> = Vec::new();
let mut samples: u64 = 0;
let mut alerts: Vec<Alert> = Vec::new();
let mut raised: u32 = 0;
let mut cpu_limit: String = String::new();
let mut mem_limit: String = String::new();
let mut deploy_dir: String = String::new();
let mut deploy_line: String = "no deploy run from this window yet".to_string();

let mut gpu_limit: String = "1".to_string();
let mut window_name: String = "10 MIN".to_string();

fn Page(name: String) -> Element {
    let _ = name;
    return <div key="resource" id="screen-resource">
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
    </div>;
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
        rule: format!("Added while the reading was {:.0}%", cpu),
        metric: "cpu".to_string(),
        over,
        critical: false,
        on: true,
    });
    alerts = armed;
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
