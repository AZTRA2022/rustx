// The default update wizard, shown when a different version is already
// installed. Write system/update.rsx in your project to replace it.

let mut percent: u32 = 0;
let mut working: bool = false;
let mut done: bool = false;
let mut failed: String = String::new();

fn Page() -> Element {
    let bar = format!("{}%", percent);
    let info = rustx::setup::info();
    let was = info.installed_version.clone().unwrap_or_default();
    let button = "padding:8px 18px;border-radius:8px;border:1px solid #3a3f4a;background:transparent;color:#f3f4f6;cursor:pointer;font-size:14px;";
    let primary = "padding:8px 18px;border-radius:8px;border:1px solid #6366f1;background:#6366f1;color:#fff;cursor:pointer;font-size:14px;";
    let title = if done { "Updated".to_string() } else { format!("Update {}", info.name) };
    let line = if !failed.is_empty() {
        failed.clone()
    } else if done {
        format!("{} {} is installed.", info.name, info.version)
    } else {
        format!("Version {} replaces version {}. Close {} first if it is running.", info.version, was, info.name)
    };

    return <div style={{ display: "flex", flexDirection: "column", height: "100vh", fontFamily: "system-ui, sans-serif", background: "#14161a", color: "#f3f4f6" }}>
        <main style={{ flex: "1", padding: "32px 36px 0" }}>
            <h1 style={{ fontSize: "22px", margin: "0 0 4px" }}>{title}</h1>
            <p style={{ color: "#9ca3af", margin: "0 0 20px" }}>{line}</p>
            <div style={{ height: "8px", borderRadius: "4px", background: "#2a2e36", overflow: "hidden" }}>
                <div style={{ height: "100%", width: bar, background: "#6366f1" }}></div>
            </div>
        </main>
        <footer style={{ display: "flex", justifyContent: "flex-end", gap: "8px", padding: "16px 36px 24px" }}>
            <button style={button} disabled={working && !done} onClick={move |_| { rustx::setup::quit(); }}>{if done { "Close" } else { "Cancel" }}</button>
            <button style={primary} disabled={working && !done} onClick={move |_| {
                if done {
                    if rustx::setup::launch().is_ok() { rustx::setup::quit(); }
                } else {
                    working = true;
                    task(async move {
                        let result = rustx::setup::install(|p| { percent = p as u32; });
                        match result {
                            Ok(()) => done = true,
                            Err(why) => { failed = why; working = false; }
                        }
                    });
                }
            }}>{if done { "Start" } else { "Update" }}</button>
        </footer>
    </div>;
}
