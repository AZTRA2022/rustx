// The default settings screen: what the system lets an application be asked
// about. Write system/settings.rsx in your project to replace it. Open it with
// rustx::setup::open("settings").

let mut on_login: bool = false;
let mut said: String = String::new();
let mut news: String = String::new();
let mut found: bool = false;
let mut busy: bool = false;
let mut percent: u32 = 0;
let mut updated: bool = false;


// A window without the system's frame gets a bar to move it and the three marks back.
fn Chrome() -> Element {
    if !rustx::window::is_frameless() {
        return <div></div>;
    }
    let mark = "width:40px;height:28px;border:0;background:transparent;color:var(--rx-muted);cursor:pointer;font-size:13px;";
    return <div style={{ position: "absolute", top: "0", left: "0", right: "0", height: "32px", display: "flex", alignItems: "center", zIndex: "10" }}>
        <div style={{ flex: "1", height: "32px" }} onMouseDown={move |_| { rustx::window::drag(); }}></div>
        <button aria-label="Minimize" style={mark} onClick={move |_| { rustx::window::minimize(); }}>{"\u{2013}"}</button>
        <button aria-label="Maximize" style={mark} onClick={move |_| { rustx::window::toggle_maximize(); }}>{"\u{25A1}"}</button>
        <button aria-label="Close" style={mark} onClick={move |_| { rustx::window::shut(); }}>{"\u{2715}"}</button>
    </div>;
}

fn Page() -> Element {
    let name = rustx::window::about().name;
    let supported = rustx::autostart::supported();
    let current = rustx::autostart::is_on(&name) || on_login;
    let button = "padding:8px 18px;border-radius:8px;border:1px solid var(--rx-border);background:transparent;color:var(--rx-fg);cursor:pointer;font-size:14px;";
    let label = if current { "On" } else { "Off" };
    let help = if supported { "Open this application when you sign in." } else { "This system cannot do it from here." };
    return <div style={{ display: "flex", flexDirection: "column", position: "fixed", top: "0", left: "0", right: "0", bottom: "0", zIndex: "2147483000", fontFamily: "inherit", background: "var(--rx-bg)", color: "var(--rx-fg)" }}>
        {Chrome()}
        <main style={{ flex: "1", padding: "32px 36px 0" }}>
            <h1 style={{ fontSize: "22px", margin: "0 0 20px" }}>Settings</h1>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 0", borderBottom: "1px solid var(--rx-border)" }}>
                <div>
                    <div style={{ fontWeight: "600" }}>Start with the session</div>
                    <div style={{ color: "var(--rx-muted)", fontSize: "13px" }}>{help}</div>
                </div>
                <button style={button} disabled={!supported} onClick={move |_| {
                    let name = rustx::window::about().name;
                    let next = !rustx::autostart::is_on(&name);
                    match rustx::autostart::set(next, &name) {
                        Ok(()) => { on_login = next; said = String::new(); }
                        Err(why) => said = why.to_string(),
                    }
                }}>{label}</button>
            </div>
            {if rustx::setup::updates_known() {
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 0", borderBottom: "1px solid var(--rx-border)" }}>
                    <div>
                        <div style={{ fontWeight: "600" }}>Updates</div>
                        <div style={{ color: "var(--rx-muted)", fontSize: "13px" }}>{if news.is_empty() { "Look for a newer version.".to_string() } else { news.clone() }}</div>
                    </div>
                    <button style={button} disabled={busy} onClick={move |_| {
                        if updated {
                            let _ = rustx::setup::restart();
                        } else if found {
                            busy = true;
                            task(async move {
                                match rustx::setup::install_update(|done| { percent = done as u32; news = format!("Downloading… {}%", percent); }) {
                                    Ok(()) => { updated = true; news = "Updated. Restart to use it.".to_string(); }
                                    Err(why) => news = why,
                                }
                                busy = false;
                            });
                        } else {
                            busy = true;
                            news = "Checking…".to_string();
                            task(async move {
                                match rustx::setup::check_for_update() {
                                    Ok(Some(version)) => { found = true; news = format!("Version {} is available.", version); }
                                    Ok(None) => news = "You have the latest version.".to_string(),
                                    Err(why) => news = why,
                                }
                                busy = false;
                            });
                        }
                    }}>{if updated { "Restart" } else if found { "Install" } else { "Check" }}</button>
                </div>
            } else {
                <div></div>
            }}
            <p style={{ color: "var(--rx-danger)", fontSize: "13px" }}>{said.clone()}</p>
        </main>
        <footer style={{ display: "flex", justifyContent: "flex-end", padding: "16px 36px 24px" }}>
            <button style={button} onClick={move |_| { rustx::setup::close(); }}>Close</button>
        </footer>
    </div>;
}
