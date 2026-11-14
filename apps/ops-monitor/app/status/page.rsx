// STATUS — the machine's own health, and where each service stands.
//
// Uptime and boot time come from `sysinfo`, the link from
// `rustx::os::net`, the battery or mains from `rustx::os::system`.
// The per-service rows below are the organisation's list; the row for
// the service that runs here carries this machine's real numbers.

let mut cpu: f64 = 0.0;
let mut cores: usize = 0;
let mut mem_used: u64 = 0;
let mut mem_total: u64 = 0;
let mut gpu: f64 = 0.0;
let mut gpu_reachable: bool = true;
let mut history: Vec<Sample> = Vec::new();
let mut samples: u64 = 0;
let mut link: String = "…".to_string();

let mut uptime: u64 = 0;
let mut boot: u64 = 0;
let mut power: String = "…".to_string();

fn Page() -> Element {
    // Uptime is a count, not a reading: asking once a second would be
    // a process start a second for a number that only ever goes up by
    // one. It is read when the screen opens and then ticked.
    effect((), || {
        uptime = sysinfo::System::uptime().unwrap_or(0);
        boot = sysinfo::System::boot_time().unwrap_or(0);
        // Not `Ok(power)`: a binding whose name is also the name of a
        // value of this page is rewritten by the compiler into a read
        // of that value, which is not a pattern.
        power = match rustx::os::system::power() {
            Ok(reading) => format!("{reading:?}").to_lowercase(),
            Err(_) => "unknown".to_string(),
        };
    });

    async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            uptime += 1;
        }
    };

    let memory_percent = if mem_total == 0 { 0.0 } else { mem_used as f64 * 100.0 / mem_total as f64 };
    let healthy = services().iter().filter(|service| service.healthy).count();
    let all = services();

    return <div key="status" id="screen-status" className="pt-5">
        <div className="flex items-end">
            <div>
                <h1 className="text-[15px] font-semibold tracking-tight">Status</h1>
                <p className="mt-1 text-[11px] text-muted">
                    {format!("{healthy} of {} services healthy · this machine has been up {}.", all.len(), spell(uptime))}
                </p>
            </div>
            <span className="ml-auto flex items-center gap-2 rounded-[5px] border border-line bg-panel px-3 py-[7px] text-[10px] tracking-[0.1em]">
                {if healthy == all.len() {
                    <span className="h-[7px] w-[7px] rounded-full bg-[#5bbd7f]"></span>
                } else {
                    <span className="h-[7px] w-[7px] rounded-full bg-[#d9a13b]"></span>
                }}
                {if healthy == all.len() { "ALL SYSTEMS NORMAL" } else { "PARTIAL DEGRADATION" }}
            </span>
        </div>

        <div className="mt-5 grid grid-cols-4 gap-3">
            {Readout("UPTIME".to_string(), spell(uptime), format!("booted at epoch {}", boot))}
            {Readout("POWER".to_string(), power.clone(), "as the system reports it".to_string())}
            {Readout("LINK".to_string(), link.clone(), "the machine's own connection".to_string())}
            {Readout("ADAPTER".to_string(), if gpu_reachable { format!("{:.0}%", gpu) } else { "unreadable".to_string() }, "the graphics counter".to_string())}
        </div>

        <h2 className="mt-7 text-[10px] tracking-[0.14em] text-muted">LAST MINUTES, PROCESSOR</h2>
        <div className="mt-3 rounded-[7px] border border-line bg-panel p-4">
            <recharts::LineChart width={1130} height={150} data={history.clone()}>
                <recharts::CartesianGrid stroke="#242424" vertical={false} />
                <recharts::XAxis dataKey="at" stroke="#2b2b2b" tickLine={false} minTickGap={80} />
                <recharts::YAxis stroke="#2b2b2b" tickLine={false} domain={vec![0.0, 100.0]} width={34} unit="%" />
                <recharts::Tooltip isAnimationActive={false} cursor={false} />
                <recharts::Line type="monotone" dataKey="cpu_pct" name="CPU" stroke="#4a86d6" strokeWidth={1.5} dot={false} isAnimationActive={false} />
                <recharts::Line type="monotone" dataKey="mem_pct" name="MEMORY" stroke="#d9793b" strokeWidth={1.5} dot={false} isAnimationActive={false} />
            </recharts::LineChart>
            <p className="mt-1 text-[10px] text-faint">
                {format!("{} reading(s) · {} cores · memory at {:.0}% · processor at {:.0}%.", samples, cores, memory_percent, cpu)}
            </p>
        </div>

        <h2 className="mt-7 text-[10px] tracking-[0.14em] text-muted">BY SERVICE</h2>
        <div id="status-services" className="mt-3 overflow-hidden rounded-[7px] border border-line bg-panel">
            {all.iter().map(|service| Row(service.clone(), cpu)).collect::<Vec<_>>()}
        </div>
    </div>;
}

fn Readout(name: String, value: String, under: String) -> Element {
    return <div key={name.clone()} className="rounded-[7px] border border-line bg-panel px-4 py-3.5">
        <div className="text-[9px] tracking-[0.12em] text-faint">{name}</div>
        <div className="mt-1.5 truncate text-[15px] font-semibold">{value}</div>
        <div className="mt-1 truncate text-[10px] text-muted">{under}</div>
    </div>;
}

fn Row(service: Service, load: f64) -> Element {
    let going = format!("/services/{}", service.slug);
    let here = service.slug == "runner-gateway";
    return <button
        key={service.slug.clone()}
        className="grid w-full grid-cols-[1fr_200px_110px] items-center border-b border-linesoft px-4 py-3 text-left last:border-b-0 hover:bg-panel2"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="text-[12px]">{service.name.clone()}</span>
        <span className="flex items-center gap-[3px]">
            {(0..28).map(|index| Tick(format!("{}-{index}", service.slug), service.healthy, index)).collect::<Vec<_>>()}
        </span>
        <span className="text-right text-[10px] tabular-nums text-muted">
            {if here { format!("{:.0}% now", load) } else { "no reading".to_string() }}
        </span>
    </button>;
}

/// Twenty-eight ticks, one per recent window. A degraded service is
/// drawn with two of them amber so the row reads as a history and not
/// as a solid block.
fn Tick(name: String, healthy: bool, index: usize) -> Element {
    let colour = if healthy {
        "#2f6b45"
    } else if index == 21 || index == 22 {
        "#d9a13b"
    } else {
        "#3a3a3a"
    };
    let paint = format!("width:4px;height:14px;border-radius:1px;background-color:{colour}");
    return <span key={name} style={paint}></span>;
}

/// Seconds, spelled the way a status page spells them.
fn spell(seconds: u64) -> String {
    if seconds == 0 {
        return "a moment".to_string();
    }
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3600;
    let minutes = (seconds % 3600) / 60;
    if days > 0 {
        return format!("{days}d {hours}h {minutes}m");
    }
    if hours > 0 {
        return format!("{hours}h {minutes}m");
    }
    format!("{minutes}m {}s", seconds % 60)
}
