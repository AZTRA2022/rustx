// Vantage — the frame every screen of this app is drawn inside.
//
// The sidebar is here rather than in a page because it outlives the
// screen: moving from the overview to an agent and back keeps the
// search text, the fleet, and the scroll position of the list. The nav
// knows where it is from `rustx::router::current()`, so nothing has to
// be told which item to light up.
//
// The one piece of the machine this file touches is a real global
// shortcut: CTRL+ALT+K is held for as long as the window lives, and
// pressing it anywhere on the desktop raises the window and clears the
// search. The hint beside the search box says so.

#[derive(Clone)]
struct Fleet {
    name: String,
    slug: String,
    live: bool,
}

// `nav_` and `fleet_` prefixes: a page's values are kept by name for
// the whole application, so two files that both said `query` would be
// talking about one cell.
let mut nav_query: String = String::new();
let mut nav_fleet: Vec<Fleet> = Vec::new();
let mut nav_raises: u32 = 0;
let mut nav_shortcut_held: bool = false;

fn Layout(children: Element) -> Element {
    effect((), || {
        if nav_fleet.is_empty() {
            nav_fleet = vec![
                Fleet { name: "Support Triage".to_string(), slug: "support-triage".to_string(), live: true },
                Fleet { name: "Refund Copilot".to_string(), slug: "refund-copilot".to_string(), live: false },
                Fleet { name: "Lead Qualifier".to_string(), slug: "lead-qualifier".to_string(), live: true },
                Fleet { name: "Doc Indexer".to_string(), slug: "doc-indexer".to_string(), live: false },
                Fleet { name: "Churn Watcher".to_string(), slug: "churn-watcher".to_string(), live: false },
            ];
        }
    });

    // A real combination, held by the system for as long as this loop
    // lives — which is as long as the layout is on screen. The
    // `Shortcut` is owned by the future, so leaving hands the keys back
    // to whoever asks for them next.
    async move {
        let held = rustx::shortcut::Shortcut::listen("CTRL+ALT+K");
        nav_shortcut_held = held.is_ok();

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            let presses = match held.as_ref() {
                Ok(shortcut) => shortcut.pressed(),
                Err(_) => 0,
            };
            if presses > 0 {
                nav_query = String::new();
                nav_raises += presses as u32;
                rustx::window::restore();
            }
        }
    };

    return <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
        {Sidebar()}
        <main className="flex-1 overflow-y-auto">{children}</main>
    </div>;
}

fn Sidebar() -> Element {
    let here = rustx::router::current();
    let needle = nav_query.to_lowercase();
    let shown: Vec<Fleet> = nav_fleet
        .iter()
        .filter(|agent| needle.is_empty() || agent.name.to_lowercase().contains(&needle))
        .cloned()
        .collect();

    return <aside className="flex w-[280px] shrink-0 flex-col border-r border-border bg-[#0b0c0b] px-4 py-5">
        <div className="flex items-center gap-3 px-2">
            <span className="flex h-8 w-8 items-center justify-center rounded-[10px] bg-white">
                <Hexagon size={17} color="#0b0c0b" strokeWidth={2} />
            </span>
            <span className="text-[17px] font-semibold tracking-tight">Vantage</span>
            <ChevronsUpDown size={14} color="#6f746f" />
        </div>

        <div className="mt-5 flex items-center gap-2 rounded-xl border border-border bg-[#131413] px-3 py-2.5">
            <Search size={14} color="#6f746f" />
            <input
                id="search"
                className="w-full bg-transparent text-[13px] text-foreground placeholder:text-[#6f746f] outline-none"
                placeholder="Search"
                value={nav_query.clone()}
                onInput={move |event| nav_query = event.value}
            />
            <span className="shrink-0 text-[11px] text-[#6f746f]">⌃⌥K</span>
        </div>

        <nav className="mt-5 flex flex-col gap-0.5">
            {NavRow("Overview".to_string(), "/".to_string(), here.clone(), String::new())}
            {NavRow("Agents".to_string(), "/agents".to_string(), here.clone(), String::new())}
            {NavRow("Runs".to_string(), "/runs".to_string(), here.clone(), nav_fleet.iter().filter(|a| a.live).count().to_string())}
            {NavRow("Evals".to_string(), "/evals".to_string(), here.clone(), String::new())}
            {NavRow("Datasets".to_string(), "/datasets".to_string(), here.clone(), String::new())}
            {NavRow("Billing".to_string(), "/billing".to_string(), here.clone(), String::new())}
        </nav>

        <div className="mt-7 flex items-center px-3">
            <span className="text-[12px] text-[#6f746f]">Agents</span>
            <button className="ml-auto text-[#6f746f] hover:text-foreground" onClick={move |_| hire()}>
                <Plus size={14} color="#6f746f" />
            </button>
        </div>

        <div id="fleet" className="mt-2 flex flex-col gap-0.5 overflow-y-auto">
            {shown.iter().map(|agent| FleetRow(agent.clone(), here.clone())).collect::<Vec<_>>()}
        </div>

        <div className="mt-auto px-3 pt-4 text-[10px] leading-relaxed text-[#5d615d]">
            {if nav_shortcut_held {
                <span>CTRL+ALT+K raises this window from anywhere.</span>
            } else {
                <span>CTRL+ALT+K is taken by another application.</span>
            }}
            {if nav_raises > 0 {
                <span>{format!(" Raised {} time(s).", nav_raises)}</span>
            } else {
                <span></span>
            }}
        </div>
    </aside>;
}

fn NavRow(label: String, route: String, here: String, badge: String) -> Element {
    // One item lights up, and it is the one the address is under:
    // "Agents" owns every `/agent/...` screen, every other item owns
    // exactly its own route.
    let active = if route == "/agents" {
        here.starts_with("/agent")
    } else {
        here == route
    };
    let classes = format!(
        "flex items-center gap-3 rounded-xl px-3 py-2.5 text-[14px] {}",
        if active {
            "border border-border bg-[#161716] text-foreground"
        } else {
            "border border-transparent text-[#9aa09a] hover:text-foreground"
        }
    );
    let going = route.clone();
    return <button key={label.clone()} className={classes} onClick={move |_| rustx::router::navigate(going.clone())}>
        {NavIcon(label.clone(), active)}
        <span>{label}</span>
        {if badge.is_empty() {
            <span></span>
        } else {
            <span className="ml-auto rounded-md bg-[#1c1e1c] px-2 py-0.5 text-[11px] text-[#9aa09a]">{badge}</span>
        }}
    </button>;
}

fn NavIcon(label: String, active: bool) -> Element {
    let tint = if active { "#e8eae8".to_string() } else { "#9aa09a".to_string() };
    if label == "Overview" {
        return <LayoutGrid size={16} color={tint} />;
    }
    if label == "Agents" {
        return <Briefcase size={16} color={tint} />;
    }
    if label == "Runs" {
        return <Activity size={16} color={tint} />;
    }
    if label == "Evals" {
        return <CircleCheck size={16} color={tint} />;
    }
    if label == "Datasets" {
        return <Database size={16} color={tint} />;
    }
    return <CreditCard size={16} color={tint} />;
}

fn FleetRow(agent: Fleet, here: String) -> Element {
    let route = format!("/agent/{}", agent.slug);
    let active = here == route;
    let classes = format!(
        "flex items-center gap-3 rounded-xl px-3 py-2.5 text-[14px] {}",
        if active { "text-foreground" } else { "text-[#9aa09a] hover:text-foreground" }
    );
    let going = route.clone();
    return <button key={agent.slug.clone()} className={classes} onClick={move |_| rustx::router::navigate(going.clone())}>
        {if agent.live {
            <span className="flex h-4 w-4 items-center justify-center rounded-full border border-[#3ddc84]">
                <span className="h-[6px] w-[6px] rounded-full bg-[#3ddc84]"></span>
            </span>
        } else {
            <span className="h-4 w-4 rounded-full border border-dashed border-[#4a4f4a]"></span>
        }}
        <span>{agent.name.clone()}</span>
    </button>;
}

/// The `+` beside "Agents": a new one, named after how many there are,
/// added to the fleet the sidebar draws and the screens read.
fn hire() {
    let mut fleet = nav_fleet.clone();
    let number = fleet.len() + 1;
    fleet.push(Fleet {
        name: format!("New Agent {number}"),
        slug: format!("new-agent-{number}"),
        live: false,
    });
    nav_fleet = fleet;
}
