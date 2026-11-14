// OVERVIEW — the service's facts, and the heaviest processes on the
// machine it runs on.
//
// "Agents" in this console are the processes actually running here,
// read from `sysinfo` by the loop in the root layout. Nothing on this
// screen is invented except the service's own identity.

let mut cpu: f64 = 0.0;
let mut cores: usize = 0;
let mut mem_used: u64 = 0;
let mut mem_total: u64 = 0;
let mut gpu: f64 = 0.0;
let mut gpu_name: String = String::new();
let mut processes: Vec<(String, f64, u64)> = Vec::new();
let mut samples: u64 = 0;
let mut link: String = "…".to_string();
let mut shell_name: String = String::new();
let mut raised: u32 = 0;

fn Page(name: String) -> Element {
    let service = service_named(&name);
    let memory_percent = if mem_total == 0 { 0.0 } else { mem_used as f64 * 100.0 / mem_total as f64 };

    return <div key="overview" id="screen-overview" className="mt-5 grid grid-cols-[1fr_1fr] gap-4">
        <div className="rounded-[7px] border border-line bg-panel p-4">
            <h3 className="text-[10px] tracking-[0.14em] text-muted">SERVICE</h3>
            <div className="mt-3 grid grid-cols-2 gap-y-3">
                {Fact("Name".to_string(), service.name.clone())}
                {Fact("Kind".to_string(), service.kind.clone())}
                {Fact("Link".to_string(), link.clone())}
                {Fact("Shell".to_string(), shell_name.clone())}
                {Fact("Processor".to_string(), format!("{} cores · {:.0}%", cores, cpu))}
                {Fact("Memory".to_string(), format!("{:.0}% of {:.0} GB", memory_percent, mem_total as f64 / 1_073_741_824.0))}
                {Fact("Adapter".to_string(), if gpu_name.is_empty() { "reading…".to_string() } else { gpu_name.clone() })}
                {Fact("Adapter load".to_string(), format!("{:.0}%", gpu))}
                {Fact("Samples".to_string(), samples.to_string())}
                {Fact("Alerts raised".to_string(), raised.to_string())}
            </div>
            <button
                className="mt-5 rounded-[5px] border border-line bg-panel2 px-3 py-[6px] text-[10px] tracking-[0.1em] text-muted hover:text-foreground"
                onClick={move |_| copy_report()}
            >
                COPY MACHINE REPORT
            </button>
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

fn copy_report() {
    let report = format!(
        "runner-gateway · {} cores at {:.0}% · {:.1} of {:.1} GB resident · adapter {} at {:.0}% · link {} · {} samples",
        cores,
        cpu,
        mem_used as f64 / 1_073_741_824.0,
        mem_total as f64 / 1_073_741_824.0,
        if gpu_name.is_empty() { "unknown".to_string() } else { gpu_name.clone() },
        gpu,
        link,
        samples
    );
    let _ = rustx::clipboard::set_text(&report);
    let _ = rustx::notify::show(rustx::notify::What::new(
        "FORGE ORG",
        "The machine report is on the clipboard.",
    ));
}
