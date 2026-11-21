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
    let button = "padding:6px 14px;border-radius:8px;border:1px solid #3a3f4a;background:transparent;color:#f3f4f6;cursor:pointer;font-size:13px;";
    let asked = name.clone();
    let state = rustx::setup::permission_state(&name);
    return <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 0", borderBottom: "1px solid #2a2e36" }}>
        <div>
            <div style={{ fontWeight: "600" }}>{name}</div>
            <div style={{ color: "#9ca3af", fontSize: "13px" }}>{why}</div>
            <div style={{ color: "#9ca3af", fontSize: "12px", marginTop: "2px" }}>{state}</div>
        </div>
        <button style={button} onClick={move |_| { let _ = rustx::setup::open_permission_settings(&asked); }}>System settings</button>
    </div>;
}

fn Page() -> Element {
    let declared = rustx::setup::permissions();
    let rows: Vec<Element> = declared.iter().map(|name| Row(name.clone())).collect();
    let line = if declared.is_empty() { "This application asks for no special permission." } else { "The system asks you the first time each is used. You can change them in the system settings." };
    let button = "padding:8px 18px;border-radius:8px;border:1px solid #3a3f4a;background:transparent;color:#f3f4f6;cursor:pointer;font-size:14px;";
    return <div style={{ display: "flex", flexDirection: "column", height: "100vh", fontFamily: "system-ui, sans-serif", background: "#14161a", color: "#f3f4f6" }}>
        <main style={{ flex: "1", padding: "32px 36px 0", overflow: "auto" }}>
            <h1 style={{ fontSize: "22px", margin: "0 0 4px" }}>Permissions</h1>
            <p style={{ color: "#9ca3af", margin: "0 0 20px" }}>{line}</p>
            {rows}
        </main>
        <footer style={{ display: "flex", justifyContent: "flex-end", padding: "16px 36px 24px" }}>
            <button style={button} onClick={move |_| { rustx::setup::close(); }}>Close</button>
        </footer>
    </div>;
}
