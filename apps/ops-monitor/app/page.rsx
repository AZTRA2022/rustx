// Ops Monitor.
//
// The numbers on this screen are read from the real machine with the
// `sysinfo` crate — not mocked — on a background loop that reads and
// writes the page's own state directly, the way an `async` block
// tied to a component is documented to. Crossing the CPU threshold
// raises a real desktop notification; sending an email or a WhatsApp
// message, which the showcase page's copy also promises, is out of
// reach for a demo and is left undone rather than faked. The CPU
// history is drawn with `recharts::LineChart`, off npm.

#[derive(rustx::ToValue, Clone)]
struct Point {
    at: f64,
    value: f64,
}

let mut cpu: f32 = 0.0;
let mut cpu_history: Vec<f64> = Vec::new();
let mut mem_used: u64 = 0;
let mut mem_total: u64 = 0;
let mut disk_used: u64 = 0;
let mut disk_total: u64 = 0;
let mut top: Vec<(String, f32, u64)> = Vec::new();
let mut threshold: f32 = 85.0;
let mut alerts_sent: u32 = 0;
let mut ticks: u64 = 0;

fn Page() -> Element {
    // A background loop, not a raw thread: it reads `threshold` — a
    // value of the page — straight from where it is declared, and
    // writes the rest the same way, with no `alive.post` needed. The
    // numbers come from `sysinfo`, refreshed on the machine this app
    // runs on; nothing here is a mocked number.
    async move {
        let mut system = sysinfo::System::new_all();
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let mut was_over = false;

        loop {
            system.refresh_cpu_usage();
            system.refresh_memory();
            system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

            let usage = system.global_cpu_usage();
            let used = system.used_memory();
            let total = system.total_memory();

            let mut disk_used_total: u64 = 0;
            let mut disk_total_total: u64 = 0;
            for disk in disks.list() {
                disk_total_total += disk.total_space();
                disk_used_total += disk.total_space().saturating_sub(disk.available_space());
            }

            let mut heaviest: Vec<(String, f32, u64)> = system
                .processes()
                .values()
                .map(|process| (process.name().to_string_lossy().to_string(), process.cpu_usage(), process.memory()))
                .collect();
            heaviest.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            heaviest.truncate(6);

            let over = usage > threshold;
            if over && !was_over {
                let _ = rustx::notify::show(
                    rustx::notify::What::new(
                        "Ops Monitor",
                        &format!("CPU at {:.0}%, over the {:.0}% line.", usage, threshold),
                    ),
                );
                alerts_sent += 1;
            }
            was_over = over;

            cpu = usage;
            cpu_history.push(usage as f64);
            if cpu_history.len() > 40 {
                cpu_history.remove(0);
            }
            mem_used = used;
            mem_total = total;
            disk_used = disk_used_total;
            disk_total = disk_total_total;
            top = heaviest;
            ticks += 1;

            tokio::time::sleep(std::time::Duration::from_millis(900)).await;
        }
    };

    return <div style={{ fontFamily: "Segoe UI, system-ui, sans-serif", padding: "24px", backgroundColor: "#0b0d10", color: "#e5e7eb", minHeight: "100vh" }}>
        <h1 style={{ margin: "0 0 4px", fontSize: "22px" }}>Ops Monitor</h1>
        <p style={{ margin: "0 0 20px", color: "#9ca3af", fontSize: "13px" }}>
            Remote resource app, for the agents running on this machine. {ticks} samples taken.
        </p>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "14px" }}>
            {Tile("CPU".to_string(), format!("{:.0}%", cpu), cpu as f64 / 100.0, cpu > threshold)}
            {Tile("RAM".to_string(), format!("{} / {}", gb(mem_used), gb(mem_total)), ratio(mem_used, mem_total), false)}
            {Tile("Disk".to_string(), format!("{} / {}", gb(disk_used), gb(disk_total)), ratio(disk_used, disk_total), false)}
        </div>

        <div style={{ marginTop: "20px", display: "flex", alignItems: "center", gap: "12px", color: "#9ca3af", fontSize: "13px" }}>
            <label htmlFor="threshold">alert above</label>
            <input id="threshold" type="range" min="10" max="100" value={threshold.to_string()}
                onInput={move |event| threshold = event.number().unwrap_or(85.0) as f32} />
            <span id="threshold-value">{format!("{:.0}%", threshold)}</span>
            <span id="alerts-sent" style={{ marginLeft: "auto" }}>{alerts_sent} alert(s) sent</span>
        </div>

        <div style={{ marginTop: "20px", border: "1px solid #1f2430", borderRadius: "12px", padding: "16px", backgroundColor: "#11151c" }}>
            <h2 style={{ margin: "0 0 10px", fontSize: "12px", color: "#9ca3af", textTransform: "uppercase", letterSpacing: "1px" }}>cpu history</h2>
            <recharts::LineChart width={740} height={160} data={cpu_history.iter().enumerate().map(|(at, value)| Point { at: at as f64, value: *value }).collect::<Vec<_>>()}>
                <recharts::XAxis dataKey="at" stroke="#52525b" />
                <recharts::YAxis stroke="#52525b" domain={vec![0.0, 100.0]} />
                <recharts::Line type="monotone" dataKey="value" stroke="#60a5fa" strokeWidth={2} isAnimationActive={false} dot={false} />
            </recharts::LineChart>
        </div>

        <h2 style={{ marginTop: "28px", fontSize: "14px", color: "#9ca3af", textTransform: "uppercase", letterSpacing: "1px" }}>
            heaviest agents
        </h2>
        <table id="processes" style={{ width: "100%", marginTop: "10px", borderCollapse: "collapse", fontSize: "13px" }}>
            <tbody>
                {top.iter().map(|row| ProcessRow(row.0.clone(), row.1, row.2)).collect::<Vec<_>>()}
            </tbody>
        </table>
    </div>;
}

fn Tile(label: String, value: String, ratio: f64, hot: bool) -> Element {
    let bar_color = if hot { "#f87171" } else { "#60a5fa" };
    // A `style={{ … }}` object is read at compile time and takes only
    // literal strings, so anything computed — a width, a colour that
    // depends on the number — is written as the declaration itself.
    let bar = format!(
        "height: 100%; width: {}%; background-color: {}",
        (ratio.clamp(0.0, 1.0) * 100.0) as u32,
        bar_color
    );
    return <div style={{ border: "1px solid #1f2430", borderRadius: "12px", padding: "16px", backgroundColor: "#11151c" }}>
        <div style={{ fontSize: "12px", color: "#9ca3af", textTransform: "uppercase", letterSpacing: "1px" }}>{label}</div>
        <div style={{ fontSize: "24px", fontWeight: "600", marginTop: "6px" }}>{value}</div>
        <div style={{ marginTop: "10px", height: "6px", borderRadius: "3px", backgroundColor: "#1f2430", overflow: "hidden" }}>
            <div style={bar}></div>
        </div>
    </div>;
}

fn ProcessRow(process_name: String, cpu_percent: f32, memory: u64) -> Element {
    return <tr key={process_name.clone()} style={{ borderBottom: "1px solid #1f2430" }}>
        <td style={{ padding: "6px 4px" }}>{process_name}</td>
        <td style={{ padding: "6px 4px", textAlign: "right" }}>{format!("{:.1}%", cpu_percent)}</td>
        <td style={{ padding: "6px 4px", textAlign: "right", color: "#9ca3af" }}>{gb(memory)}</td>
    </tr>;
}

fn gb(bytes: u64) -> String {
    format!("{:.2} GB", bytes as f64 / 1_000_000_000.0)
}

fn ratio(used: u64, total: u64) -> f64 {
    if total == 0 { 0.0 } else { used as f64 / total as f64 }
}
