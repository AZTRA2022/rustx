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
    let button = "padding:8px 18px;border-radius:8px;border:1px solid var(--rx-border);background:transparent;color:var(--rx-fg);cursor:pointer;font-size:14px;";
    let primary = "padding:8px 18px;border-radius:8px;border:1px solid var(--rx-fg);background:var(--rx-fg);color:var(--rx-bg);cursor:pointer;font-size:14px;";
    let title = if done { "Updated".to_string() } else { format!("Update {}", info.name) };
    let line = if !failed.is_empty() {
        failed.clone()
    } else if done {
        format!("{} {} is installed.", info.name, info.version)
    } else {
        format!("Version {} replaces version {}. Close {} first if it is running.", info.version, was, info.name)
    };

    return <div style={{ display: "flex", flexDirection: "column", position: "fixed", top: "0", left: "0", right: "0", bottom: "0", zIndex: "2147483000", fontFamily: "inherit", background: "var(--rx-bg)", color: "var(--rx-fg)" }}>
        <main style={{ flex: "1", padding: "32px 36px 0" }}>
            <h1 style={{ fontSize: "22px", margin: "0 0 4px" }}>{title}</h1>
            <p style={{ color: "var(--rx-muted)", margin: "0 0 20px" }}>{line}</p>
            <div style={{ height: "8px", borderRadius: "4px", background: "var(--rx-border)", overflow: "hidden" }}>
                <div style={{ height: "100%", width: bar, background: "var(--rx-fg)" }}></div>
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
