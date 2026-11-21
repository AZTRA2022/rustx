// The default settings screen: what the system lets an application be asked
// about. Write system/settings.rsx in your project to replace it. Open it with
// rustx::setup::open("settings").

let mut on_login: bool = false;
let mut said: String = String::new();

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
            <p style={{ color: "#f87171", fontSize: "13px" }}>{said.clone()}</p>
        </main>
        <footer style={{ display: "flex", justifyContent: "flex-end", padding: "16px 36px 24px" }}>
            <button style={button} onClick={move |_| { rustx::setup::close(); }}>Close</button>
        </footer>
    </div>;
}
