// The default permissions screen: what the application uses, said in words
// before the system asks. Declare them with Config::permissions(&["camera"]).
// Write system/permissions.rsx in your project to replace it.

fn Row(name: String) -> Element {
    let why = if name == "camera" {
        "to show and record what the camera sees"
    } else if name == "microphone" {
        "to record sound"
    } else if name == "location" {
        "to know where you are"
    } else if name == "notifications" {
        "to tell you when something happens"
    } else {
        "to read and write the files you choose"
    };
    let button = "padding:6px 14px;border-radius:8px;border:1px solid var(--rx-border);background:transparent;color:var(--rx-fg);cursor:pointer;font-size:13px;";
    let asked = name.clone();
    let state = rustx::setup::permission_state(&name);
    return <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 0", borderBottom: "1px solid var(--rx-border)" }}>
        <div>
            <div style={{ fontWeight: "600" }}>{name}</div>
            <div style={{ color: "var(--rx-muted)", fontSize: "13px" }}>{why}</div>
            <div style={{ color: "var(--rx-muted)", fontSize: "12px", marginTop: "2px" }}>{state}</div>
        </div>
        <button style={button} onClick={move |_| { let _ = rustx::setup::open_permission_settings(&asked); }}>System settings</button>
    </div>;
}


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
    let declared = rustx::setup::permissions();
    let rows: Vec<Element> = declared.iter().map(|name| Row(name.clone())).collect();
    let line = if declared.is_empty() { "This application asks for no special permission." } else { "The system asks you the first time each is used. You can change them in the system settings." };
    let button = "padding:8px 18px;border-radius:8px;border:1px solid var(--rx-border);background:transparent;color:var(--rx-fg);cursor:pointer;font-size:14px;";
    return <div style={{ display: "flex", flexDirection: "column", position: "fixed", top: "0", left: "0", right: "0", bottom: "0", zIndex: "2147483000", fontFamily: "inherit", background: "var(--rx-bg)", color: "var(--rx-fg)" }}>
        {Chrome()}
        <main style={{ flex: "1", padding: "32px 36px 0", overflow: "auto" }}>
            <h1 style={{ fontSize: "22px", margin: "0 0 4px" }}>Permissions</h1>
            <p style={{ color: "var(--rx-muted)", margin: "0 0 20px" }}>{line}</p>
            {rows}
        </main>
        <footer style={{ display: "flex", justifyContent: "flex-end", padding: "16px 36px 24px" }}>
            <button style={button} onClick={move |_| { rustx::setup::close(); }}>Close</button>
        </footer>
    </div>;
}
