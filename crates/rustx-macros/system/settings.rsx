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

fn Page() -> Element {
    let name = rustx::window::about().name;
    let supported = rustx::autostart::supported();
    let current = rustx::autostart::is_on(&name) || on_login;
    let button = "padding:8px 18px;border-radius:8px;border:1px solid #3a3f4a;background:transparent;color:#f3f4f6;cursor:pointer;font-size:14px;";
    let label = if current { "On" } else { "Off" };
    let help = if supported { "Open this application when you sign in." } else { "This system cannot do it from here." };
    return <div style={{ display: "flex", flexDirection: "column", height: "100vh", fontFamily: "system-ui, sans-serif", background: "#14161a", color: "#f3f4f6" }}>
        <main style={{ flex: "1", padding: "32px 36px 0" }}>
            <h1 style={{ fontSize: "22px", margin: "0 0 20px" }}>Settings</h1>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 0", borderBottom: "1px solid #2a2e36" }}>
                <div>
                    <div style={{ fontWeight: "600" }}>Start with the session</div>
                    <div style={{ color: "#9ca3af", fontSize: "13px" }}>{help}</div>
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
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 0", borderBottom: "1px solid #2a2e36" }}>
                    <div>
                        <div style={{ fontWeight: "600" }}>Updates</div>
                        <div style={{ color: "#9ca3af", fontSize: "13px" }}>{if news.is_empty() { "Look for a newer version.".to_string() } else { news.clone() }}</div>
                    </div>
                    <button style={button} disabled={busy} onClick={move |_| {
                        if updated {
                            let _ = rustx::setup::restart();
                        } else if found {
                            task(async move {
                                busy = true;
                                match rustx::setup::install_update(|done| { percent = done as u32; news = format!("Downloading… {}%", percent); }) {
                                    Ok(()) => { updated = true; news = "Updated. Restart to use it.".to_string(); }
                                    Err(why) => news = why,
                                }
                                busy = false;
                            });
                        } else {
                            task(async move {
                                busy = true;
                                news = "Checking…".to_string();
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
            <p style={{ color: "#f87171", fontSize: "13px" }}>{said.clone()}</p>
        </main>
        <footer style={{ display: "flex", justifyContent: "flex-end", padding: "16px 36px 24px" }}>
            <button style={button} onClick={move |_| { rustx::setup::close(); }}>Close</button>
        </footer>
    </div>;
}
