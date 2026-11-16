// The frame every screen of the console sits in: the organisation bar
// at the top, and the two loops that read this machine.
//
// The loops live here rather than in a page because the readings
// outlive the screen — moving from RESOURCE to ANALYTICS must not
// restart the history. A value a page declares is kept by its name for
// the whole application, so a page below redeclares the handful of
// values it reads and gets the same cells these loops write.

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

let mut alerts: Vec<Alert> = Vec::new();
let mut raised: u32 = 0;
let mut cpu_limit: String = String::new();
let mut mem_limit: String = String::new();
let mut deploy_dir: String = String::new();
let mut shell_name: String = String::new();



fn Layout(children: Element) -> Element {
    effect((), || {
        if alerts.is_empty() {
            alerts = starting_alerts();
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
    // so arming a rule from a screen changes what this loop watches on
    // its next pass, with nothing posted between them.
    async move {
        let mut system = sysinfo::System::new_all().unwrap();
        let mut over: [bool; 12] = [false; 12];

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
            && output.worked()
        {
            let line = output.out.trim().to_string();
            if let Some((name, vram)) = line.split_once('|') {
                gpu_name = name.trim().to_string();
                gpu_vram = vram.trim().parse::<u64>().unwrap_or(0);
            } else if !line.is_empty() {
                gpu_name = line;
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
        <div className="mx-auto w-full max-w-[1240px] px-7 pb-10">{children}</div>
    </div>;
}

fn TopBar() -> Element {
    let here = rustx::router::current();
    return <header className="flex h-11 items-center gap-6 border-b border-line bg-panel px-5">
        <button className="flex items-center gap-2" onClick={move |_| rustx::router::navigate("/".to_string())}>
            <span className="flex h-[18px] w-[18px] items-center justify-center rounded-[4px] bg-brand text-[10px] font-bold text-black">F</span>
            <span className="text-[12px] font-semibold tracking-[0.14em]">FORGE ORG</span>
            <span className="rounded-[3px] bg-[#2f2f2f] px-1.5 py-[1px] text-[9px] tracking-[0.12em] text-muted">PRO</span>
            <ChevronDown size={11} color="#6a6a66" />
        </button>
        <nav className="ml-auto flex items-center gap-1">
            {NavItem("Home".to_string(), "/".to_string(), here.clone())}
            {NavItem("Services".to_string(), "/services".to_string(), here.clone())}
            {NavItem("Tasks".to_string(), "/tasks".to_string(), here.clone())}
            {NavItem("Docs".to_string(), "/docs".to_string(), here.clone())}
            {NavItem("Status".to_string(), "/status".to_string(), here.clone())}
            {NavItem("Changelog".to_string(), "/changelog".to_string(), here.clone())}
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

fn NavItem(label: String, route: String, here: String) -> Element {
    let active = if route == "/" { here == "/" } else { here.starts_with(&route) };
    let classes = format!(
        "rounded-[4px] px-2.5 py-1 text-[11px] {}",
        if active { "bg-[#2a2a2a] text-foreground" } else { "text-muted hover:text-foreground" }
    );
    let going = route.clone();
    return <button key={label.clone()} className={classes} onClick={move |_| rustx::router::navigate(going.clone())}>
        {label}
    </button>;
}

fn starting_alerts() -> Vec<Alert> {
    vec![
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
    ]
}

/// The wall clock, as a chart's x axis reads it.
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
