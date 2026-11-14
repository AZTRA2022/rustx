// SERVICES — the list the breadcrumb of a service detail points back
// to, and the only way into the four detail screens that is not a
// typed address.

let mut cpu: f64 = 0.0;
let mut mem_used: u64 = 0;
let mut mem_total: u64 = 0;
let mut gpu: f64 = 0.0;
let mut service_filter: String = String::new();

fn Page() -> Element {
    let needle = service_filter.to_lowercase();
    let shown: Vec<Service> = services()
        .into_iter()
        .filter(|service| {
            needle.is_empty()
                || service.name.to_lowercase().contains(&needle)
                || service.kind.to_lowercase().contains(&needle)
        })
        .collect();

    return <div key="services" id="screen-services" className="pt-5">
        <div className="flex items-center gap-2 py-1 text-[11px] text-muted">
            <LayoutGrid size={11} color="#6a6a66" />
            <span className="text-foreground">Services</span>
        </div>

        <div className="mt-3 flex items-end">
            <div>
                <h1 className="text-[15px] font-semibold tracking-tight">All services</h1>
                <p className="mt-1 text-[11px] text-muted">
                    The first one runs on this machine; the rest are named but not read.
                </p>
            </div>
            <span className="ml-auto flex items-center gap-2 rounded-[5px] border border-line bg-[#131313] px-2.5 py-[6px]">
                <Search size={11} color="#6a6a66" />
                <input
                    id="service-filter"
                    className="w-48 bg-transparent text-[11px] text-foreground outline-none placeholder:text-faint"
                    placeholder="Filter services"
                    value={service_filter.clone()}
                    onInput={move |event| service_filter = event.value}
                />
            </span>
        </div>

        <div id="service-list" className="mt-4 overflow-hidden rounded-[7px] border border-line bg-panel">
            <div className="grid grid-cols-[1fr_120px_90px_110px] border-b border-line px-4 py-2.5 text-[9px] tracking-[0.12em] text-faint">
                <span>SERVICE</span>
                <span>KIND</span>
                <span>HEALTH</span>
                <span className="text-right">LOAD</span>
            </div>
            {shown.iter().map(|service| Row(service.clone())).collect::<Vec<_>>()}
            {if shown.is_empty() {
                <div className="px-4 py-6 text-[11px] text-faint">Nothing matches that filter.</div>
            } else {
                <span></span>
            }}
        </div>
    </div>;
}

/// The load column: the machine's own numbers for the service that
/// actually runs here, and a dash for the ones that do not.
fn Row(service: Service) -> Element {
    let here = service.slug == "runner-gateway";
    let memory_percent = if mem_total == 0 { 0.0 } else { mem_used as f64 * 100.0 / mem_total as f64 };
    let load = if here {
        format!("{:.0}% · {:.0}% · {:.0}%", cpu, gpu, memory_percent)
    } else {
        "not read here".to_string()
    };
    let going = format!("/services/{}", service.slug);

    return <button
        key={service.slug.clone()}
        className="grid w-full grid-cols-[1fr_120px_90px_110px] items-center border-b border-linesoft px-4 py-3 text-left last:border-b-0 hover:bg-panel2"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="min-w-0 pr-4">
            <span className="block text-[12px]">{service.name.clone()}</span>
            <span className="mt-0.5 block truncate text-[10px] text-muted">{service.summary.clone()}</span>
        </span>
        <span className="text-[10px] tracking-[0.08em] text-muted">{service.kind.clone()}</span>
        <span>
            {if service.healthy {
                <span className="rounded-[3px] bg-[#16331f] px-1.5 py-[2px] text-[9px] tracking-[0.1em] text-[#5bbd7f]">ACTIVE</span>
            } else {
                <span className="rounded-[3px] bg-[#331a16] px-1.5 py-[2px] text-[9px] tracking-[0.1em] text-[#e0533c]">DEGRADED</span>
            }}
        </span>
        <span className="text-right text-[10px] tabular-nums text-muted">{load}</span>
    </button>;
}
