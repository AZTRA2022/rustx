// Everything a service's four screens have in common: the breadcrumb
// back to the list, the title with its pills, the description, and the
// OVERVIEW / RESOURCE / ANALYTICS / SETTINGS tabs — which are four
// routes and not four states, so each one is a page of its own and the
// address says which one you are on.
//
// A layout may take the dynamic segments above it; `name` here is the
// `[name]` of the directory this file sits in.

let mut deploy_line: String = "no deploy run from this window yet".to_string();
let mut deploy_dir: String = String::new();
let mut link: String = "…".to_string();

// A single element and not a fragment: a nested layout whose root is
// a fragment leaves its last page's nodes in the document when the
// address moves out from under it, so every screen of this console
// sits inside one node the router can replace whole.
fn Layout(children: Element, name: String) -> Element {
    let service = service_named(&name);

    return <div key="service-frame">
        <div className="flex items-center gap-2 py-3 text-[11px] text-muted">
            <LayoutGrid size={11} color="#6a6a66" />
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/services".to_string())}>
                Services
            </button>
            <span className="text-faint">/</span>
            <span className="text-foreground">Service Details</span>
        </div>

        <div className="flex items-start gap-3">
            <span className="mt-[2px] flex h-8 w-8 items-center justify-center rounded-[6px] bg-brand">
                <Cog size={17} color="#140b06" />
            </span>
            <div>
                <h1 id="service-title" className="text-[15px] font-semibold tracking-tight">{service.name.clone()}</h1>
                <div className="mt-1.5 flex items-center gap-1.5">
                    {if service.healthy {
                        <span className="rounded-[3px] bg-[#16331f] px-1.5 py-[2px] text-[9px] tracking-[0.1em] text-[#5bbd7f]">ACTIVE</span>
                    } else {
                        <span className="rounded-[3px] bg-[#331a16] px-1.5 py-[2px] text-[9px] tracking-[0.1em] text-[#e0533c]">DEGRADED</span>
                    }}
                    {service.kind.split(" · ").map(|word| Chip(word.to_string())).collect::<Vec<_>>()}
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
        </div>

        <div className="mt-4">
            <div className="text-[10px] tracking-[0.12em] text-muted">Description</div>
            <p className="mt-1.5 max-w-[560px] text-[12px] leading-[1.65] text-foreground">{service.summary.clone()}</p>
            <p id="deploy-line" className="mt-1.5 text-[11px] text-faint">{deploy_line.clone()}</p>
        </div>

        <div className="mt-4 flex w-fit rounded-[5px] border border-line bg-panel p-[3px]">
            {Tab("OVERVIEW".to_string(), format!("/services/{name}"))}
            {Tab("RESOURCE".to_string(), format!("/services/{name}/resource"))}
            {Tab("ANALYTICS".to_string(), format!("/services/{name}/analytics"))}
            {Tab("SETTINGS".to_string(), format!("/services/{name}/settings"))}
        </div>

        {children}
    </div>;
}

fn Chip(label: String) -> Element {
    return <span key={label.clone()} className="rounded-[3px] border border-line px-1.5 py-[2px] text-[9px] tracking-[0.1em] text-muted">
        {label}
    </span>;
}

fn Tab(label: String, route: String) -> Element {
    let here = rustx::router::current();
    let active = here == route;
    let classes = format!(
        "rounded-[3px] px-3 py-[6px] text-[10px] tracking-[0.11em] {}",
        if active { "bg-[#2e2e2e] text-foreground" } else { "text-muted hover:text-foreground" }
    );
    let going = route.clone();
    return <button key={label.clone()} className={classes} onClick={move |_| rustx::router::navigate(going.clone())}>
        {label}
    </button>;
}

/// A real terminal, in the deploy directory, left to the user.
fn open_a_terminal() {
    let at = std::path::PathBuf::from(deploy_dir.clone());
    let at = if at.is_dir() { at } else { std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")) };
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

/// The deploy. A real script, run in the real shell: it prints where it
/// ran and what the machine calls itself, and its exit code is the one
/// the console shows.
#[cfg(windows)]
const DEPLOY_SCRIPT: &str = "\"deployed from $(Get-Location) on $env:COMPUTERNAME at $(Get-Date -Format HH:mm:ss)\"";
#[cfg(not(windows))]
const DEPLOY_SCRIPT: &str = "echo \"deployed from $(pwd) on $(hostname) at $(date +%H:%M:%S)\"";
