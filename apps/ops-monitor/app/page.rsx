// HOME — the organisation, above any one service.
//
// The four numbers across the top are this machine's, read by the loop
// in the root layout and redeclared here by name. The service cards
// below are the organisation's own list, and every one of them is a
// link into `/services/<slug>`: nothing on this screen is a dead end.

let mut cpu: f64 = 0.0;
let mut cores: usize = 0;
let mut mem_used: u64 = 0;
let mut mem_total: u64 = 0;
let mut gpu: f64 = 0.0;
let mut gpu_name: String = String::new();
let mut samples: u64 = 0;
let mut link: String = "…".to_string();
let mut raised: u32 = 0;
let mut shell_name: String = String::new();


fn Page() -> Element {
    let memory_percent = if mem_total == 0 { 0.0 } else { mem_used as f64 * 100.0 / mem_total as f64 };
    let all = services();
    let degraded = all.iter().filter(|service| !service.healthy).count();

    return <div key="home" id="screen-home" className="pt-6">
        <div className="flex items-end">
            <div>
                <h1 className="text-[18px] font-semibold tracking-tight">Good shift, Mira.</h1>
                <p className="mt-1 text-[12px] text-muted">
                    {format!(
                        "{} services under FORGE ORG · {} degraded · {} alert(s) raised since this window opened.",
                        all.len(),
                        degraded,
                        raised
                    )}
                </p>
            </div>
            <button
                className="ml-auto flex items-center gap-1.5 rounded-[5px] border border-line bg-[#262626] px-3 py-[7px] text-[10px] tracking-[0.1em] text-foreground hover:border-faint"
                onClick={move |_| rustx::router::navigate("/services".to_string())}
            >
                <LayoutGrid size={11} color="#d6d6d4" />
                ALL SERVICES
            </button>
        </div>

        <div className="mt-5 grid grid-cols-4 gap-3">
            {Tile("PROCESSOR".to_string(), format!("{:.0}%", cpu), format!("{} cores", cores), "#4a86d6".to_string())}
            {Tile("ADAPTER".to_string(), format!("{:.0}%", gpu), if gpu_name.is_empty() { "reading…".to_string() } else { gpu_name.clone() }, "#4ea972".to_string())}
            {Tile("MEMORY".to_string(), format!("{:.0}%", memory_percent), format!("{:.1} of {:.0} GB", mem_used as f64 / 1_073_741_824.0, mem_total as f64 / 1_073_741_824.0), "#d9793b".to_string())}
            {Tile("LINK".to_string(), link.clone(), format!("{} samples", samples), "#8a8a85".to_string())}
        </div>

        <h2 className="mt-7 text-[10px] tracking-[0.14em] text-muted">SERVICES</h2>
        <div id="home-services" className="mt-3 grid grid-cols-2 gap-3">
            {all.iter().map(|service| ServiceCard(service.clone())).collect::<Vec<_>>()}
        </div>

        <h2 className="mt-7 text-[10px] tracking-[0.14em] text-muted">ELSEWHERE</h2>
        <div className="mt-3 grid grid-cols-4 gap-3">
            {Shortcut("Tasks".to_string(), "What the console runs on this machine".to_string(), "/tasks".to_string())}
            {Shortcut("Docs".to_string(), "Every screen of this console, written down".to_string(), "/docs".to_string())}
            {Shortcut("Status".to_string(), "The machine's own uptime and health".to_string(), "/status".to_string())}
            {Shortcut("Changelog".to_string(), "What changed in this console".to_string(), "/changelog".to_string())}
        </div>

        <p className="mt-6 text-[10px] text-faint">
            {format!("Shell: {}. Everything with a number on it is read from this machine.", if shell_name.is_empty() { "reading…".to_string() } else { shell_name.clone() })}
        </p>
    </div>;
}

fn Tile(name: String, value: String, under: String, colour: String) -> Element {
    let ink = format!("color:{colour}");
    return <div key={name.clone()} className="rounded-[7px] border border-line bg-panel px-4 py-3.5">
        <div className="text-[9px] tracking-[0.12em] text-faint">{name}</div>
        <div className="mt-1.5 text-[20px] font-semibold tabular-nums" style={ink}>{value}</div>
        <div className="mt-1 truncate text-[10px] text-muted">{under}</div>
    </div>;
}

fn ServiceCard(service: Service) -> Element {
    let going = format!("/services/{}", service.slug);
    return <button
        key={service.slug.clone()}
        className="rounded-[7px] border border-line bg-panel p-4 text-left hover:border-faint"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="flex items-center gap-2">
            <span className="flex h-6 w-6 items-center justify-center rounded-[5px] bg-brand">
                <Cog size={13} color="#140b06" />
            </span>
            <span className="text-[13px] font-semibold">{service.name.clone()}</span>
            {if service.healthy {
                <span className="ml-auto rounded-[3px] bg-[#16331f] px-1.5 py-[2px] text-[9px] tracking-widest text-[#5bbd7f]">ACTIVE</span>
            } else {
                <span className="ml-auto rounded-[3px] bg-[#331a16] px-1.5 py-[2px] text-[9px] tracking-[0.1em] text-[#e0533c]">DEGRADED</span>
            }}
        </span>
        <span className="mt-2 block text-[11px] leading-[1.6] text-muted">{service.summary.clone()}</span>
        <span className="mt-2.5 block text-[9px] tracking-[0.1em] text-faint">{service.kind.clone()}</span>
    </button>;
}

fn Shortcut(name: String, under: String, route: String) -> Element {
    let going = route.clone();
    return <button
        key={name.clone()}
        className="rounded-[7px] border border-line bg-panel px-4 py-3 text-left hover:border-faint"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="flex items-center gap-1.5 text-[12px]">
            {name}
            <ChevronRight size={11} color="#6a6a66" />
        </span>
        <span className="mt-1 block text-[10px] leading-snug text-muted">{under}</span>
    </button>;
}
