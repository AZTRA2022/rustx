// SETTINGS — the fourth tab, and the one that actually changes the
// machine rather than describing it.
//
// Start with the session is a real registry entry through
// `rustx::autostart`. The deploy directory is the one LIVE LOGS and
// NEW DEPLOY use, chosen with the system's own folder picker. The
// notification button raises a real one. Nothing here is a mock-up of
// a setting.

let mut deploy_dir: String = String::new();
let mut deploy_line: String = "no deploy run from this window yet".to_string();
let mut alerts: Vec<Alert> = Vec::new();
let mut raised: u32 = 0;
let mut shell_name: String = String::new();
let mut link: String = "…".to_string();

let mut autostart_on: bool = false;
let mut tray_on: bool = false;
let mut facts: Vec<(String, String)> = Vec::new();

fn Page(name: String) -> Element {
    let service = service_named(&name);

    effect((), || {
        autostart_on = rustx::autostart::is_on("FORGE ORG");
        facts = read_machine();
    });

    return <div key="settings" id="screen-settings">
        <h2 className="mt-5 text-[10px] tracking-[0.14em] text-muted">SETTINGS</h2>

        <div className="mt-3 grid grid-cols-[1fr_1fr] gap-4">
            <div className="rounded-[7px] border border-line bg-panel p-4">
                <h3 className="text-[11px] font-semibold">This console on this machine</h3>

                {Switch(
                    "Start with the session".to_string(),
                    if rustx::autostart::supported() {
                        "A real entry under the system's own startup list.".to_string()
                    } else {
                        "This platform does not offer one from here.".to_string()
                    },
                    autostart_on,
                    0,
                )}

                {Switch(
                    "Keep the window above the others".to_string(),
                    "A real order to the window manager, not a style.".to_string(),
                    tray_on,
                    1,
                )}

                <div className="mt-4 border-t border-linesoft pt-3.5">
                    <div className="text-[11px]">Deploy directory</div>
                    <p className="mt-1 text-[10px] leading-snug text-muted">
                        Where LIVE LOGS opens a terminal and NEW DEPLOY runs its script.
                    </p>
                    <div className="mt-2 flex items-center gap-2">
                        <span id="deploy-dir" className="min-w-0 flex-1 truncate rounded-[4px] border border-line bg-[#131313] px-2.5 py-[6px] text-[10px] text-foreground">
                            {if deploy_dir.is_empty() { "not chosen yet".to_string() } else { deploy_dir.clone() }}
                        </span>
                        <button
                            className="shrink-0 rounded-[5px] border border-line bg-panel2 px-3 py-[6px] text-[10px] tracking-[0.1em] text-muted hover:text-foreground"
                            onClick={move |_| choose_deploy_dir()}
                        >
                            CHOOSE
                        </button>
                    </div>
                </div>

                <div className="mt-4 flex gap-2 border-t border-linesoft pt-3.5">
                    <button
                        className="rounded-[5px] border border-line bg-panel2 px-3 py-[6px] text-[10px] tracking-[0.1em] text-muted hover:text-foreground"
                        onClick={move |_| test_notification()}
                    >
                        SEND A TEST ALERT
                    </button>
                    <button
                        className="rounded-[5px] border border-line bg-panel2 px-3 py-[6px] text-[10px] tracking-[0.1em] text-muted hover:text-foreground"
                        onClick={move |_| { let _ = rustx::os::attention::badge(None); raised = 0; deploy_line = "the badge is cleared".to_string(); }}
                    >
                        CLEAR THE BADGE
                    </button>
                </div>
            </div>

            <div className="rounded-[7px] border border-line bg-panel p-4">
                <h3 className="text-[11px] font-semibold">{format!("{} — what the console holds", service.name)}</h3>
                <div className="mt-3 flex flex-col gap-2">
                    {Fact("Kind".to_string(), service.kind.clone())}
                    {Fact("Health".to_string(), if service.healthy { "active".to_string() } else { "degraded".to_string() })}
                    {Fact("Link".to_string(), link.clone())}
                    {Fact("Shell".to_string(), if shell_name.is_empty() { "reading…".to_string() } else { shell_name.clone() })}
                    {Fact("Rules watched".to_string(), format!("{} ({} armed)", alerts.len(), alerts.iter().filter(|alert| alert.on).count()))}
                    {Fact("Notifications raised".to_string(), raised.to_string())}
                    // {facts.iter().map(|fact| Fact(fact.0.clone(), fact.1.clone())).collect::<Vec<_>>()}
                </div>

                <div className="mt-4 border-t border-linesoft pt-3.5">
                    <div className="text-[11px]">Rules</div>
                    <div className="mt-2 flex flex-col gap-1.5">
                        {alerts.iter().enumerate().map(|(index, alert)| RuleLine(index, alert.clone())).collect::<Vec<_>>()}
                    </div>
                    <p className="mt-3 text-[10px] text-faint">
                        A rule is armed from the RESOURCE tab; this list is what the watching loop reads.
                    </p>
                </div>
            </div>
        </div>
    </div>;
}

fn Switch(title: String, under: String, on: bool, which: usize) -> Element {
    let track = format!(
        "width:30px;height:15px;border-radius:9px;display:inline-flex;align-items:center;padding:2px;flex:none;background-color:{}",
        if on { "#4a4a4a" } else { "#262626" }
    );
    let knob = format!(
        "width:11px;height:11px;border-radius:50%;background-color:#e6e6e4;transform:translateX({})",
        if on { "15px" } else { "0px" }
    );
    return <div key={title.clone()} className="mt-3.5 flex items-start gap-3">
        <span className="min-w-0">
            <span className="block text-[11px]">{title}</span>
            <span className="mt-0.5 block text-[10px] leading-snug text-muted">{under}</span>
        </span>
        <button className="ml-auto mt-0.5" style={track} onClick={move |_| flip(which)}>
            <span style={knob}></span>
        </button>
    </div>;
}

fn Fact(name: String, value: String) -> Element {
    return <div key={name.clone()} className="flex items-baseline gap-3 text-[11px]">
        <span className="shrink-0 text-muted">{name}</span>
        <span className="ml-auto min-w-0 truncate text-right">{value}</span>
    </div>;
}

fn RuleLine(index: usize, alert: Alert) -> Element {
    let ink = if alert.on { "text-[#9fb8dc]" } else { "text-faint" };
    let classes = format!("flex items-baseline gap-2 text-[10px] {ink}");
    return <div key={index.to_string()} className={classes}>
        <span className="truncate">{alert.title.clone()}</span>
        <span className="ml-auto shrink-0">{if alert.on { "armed" } else { "off" }}</span>
    </div>;
}

// ---- what the buttons do ---------------------------------------------

fn flip(which: usize) {
    if which == 0 {
        let wanted = !autostart_on;
        match rustx::autostart::set(wanted, "FORGE ORG") {
            Ok(()) => {
                autostart_on = rustx::autostart::is_on("FORGE ORG");
                deploy_line = if autostart_on {
                    "the console now starts with the session".to_string()
                } else {
                    "the console no longer starts with the session".to_string()
                };
            }
            Err(error) => deploy_line = format!("the system refused: {error}"),
        }
        return;
    }

    let wanted = !tray_on;
    rustx::window::on_top(wanted);
    tray_on = wanted;
    deploy_line = if wanted {
        "the window is held above the others".to_string()
    } else {
        "the window is back in the stack".to_string()
    };
}

fn choose_deploy_dir() {
    match rustx::dialog::folder(rustx::dialog::Ask::new("Deploy directory")) {
        Ok(path) => {
            deploy_dir = path.display().to_string();
            deploy_line = format!("deploy directory is now {}", path.display());
        }
        Err(_) => deploy_line = "the folder picker was closed".to_string(),
    }
}

fn test_notification() {
    match rustx::notify::show(rustx::notify::What::new(
        "FORGE ORG",
        "This is what an alert looks like when a rule crosses.",
    )) {
        Ok(()) => {
            raised += 1;
            let _ = rustx::os::attention::badge(Some(raised));
            deploy_line = "a test alert was raised".to_string();
        }
        Err(error) => deploy_line = format!("the notification failed: {error}"),
    }
}

fn read_machine() -> Vec<(String, String)> {
    let theme = match rustx::os::system::theme() {
        Ok(theme) => format!("{theme:?}").to_lowercase(),
        Err(_) => "unknown".to_string(),
    };
    vec![
        ("System theme".to_string(), theme),
        ("Locale".to_string(), rustx::os::system::locale().unwrap_or_else(|_| "unknown".to_string())),
        ("Timezone".to_string(), rustx::os::system::timezone().unwrap_or_else(|_| "unknown".to_string())),
    ]
}
