// The default install wizard. Write system/install.rsx in your project to
// replace it: the steps are the state of this page, and rustx::setup does the
// work (info, install, launch, quit).

let mut step: u32 = 0;
let mut percent: u32 = 0;
let mut failed: String = String::new();

fn Heading(title: String, line: String) -> Element {
    return <div>
        <h1 style={{ fontSize: "22px", margin: "0 0 4px" }}>{title}</h1>
        <p style={{ color: "#9ca3af", margin: "0 0 20px" }}>{line}</p>
    </div>;
}

fn Welcome() -> Element {
    let info = rustx::setup::info();
    return <div>
        {Heading(format!("Install {}", info.name), format!("Version {} · {}", info.version, info.publisher))}
        <p style={{ color: "#9ca3af", fontSize: "13px", margin: "0 0 6px" }}>Install in</p>
        <div style={{ padding: "8px 12px", border: "1px solid #2a2e36", borderRadius: "8px", wordBreak: "break-all" }}>{info.destination.clone()}</div>
    </div>;
}

fn Licence() -> Element {
    let info = rustx::setup::info();
    return <div>
        {Heading("Licence".to_string(), "Read it, then accept to continue.".to_string())}
        <pre style={{ maxHeight: "240px", overflow: "auto", padding: "12px", border: "1px solid #2a2e36", borderRadius: "8px", whiteSpace: "pre-wrap", fontSize: "12px" }}>{info.license.clone()}</pre>
    </div>;
}

fn Progress() -> Element {
    let bar = format!("{}%", percent);
    return <div>
        {Heading("Installing…".to_string(), "This takes a moment.".to_string())}
        <div style={{ height: "8px", borderRadius: "4px", background: "#2a2e36", overflow: "hidden" }}>
            <div style={{ height: "100%", width: bar, background: "#6366f1" }}></div>
        </div>
    </div>;
}

fn Done() -> Element {
    let info = rustx::setup::info();
    return Heading("Ready".to_string(), format!("{} is installed.", info.name));
}

fn Failed() -> Element {
    return Heading("The installation failed".to_string(), failed.clone());
}

fn Page() -> Element {
    let has_license = !rustx::setup::info().license.is_empty();
    let button = "padding:8px 18px;border-radius:8px;border:1px solid #3a3f4a;background:transparent;color:#f3f4f6;cursor:pointer;font-size:14px;";
    let primary = "padding:8px 18px;border-radius:8px;border:1px solid #6366f1;background:#6366f1;color:#fff;cursor:pointer;font-size:14px;";
    let body = if !failed.is_empty() {
        Failed()
    } else if step == 0 {
        Welcome()
    } else if step == 1 {
        Licence()
    } else if step == 2 {
        Progress()
    } else {
        Done()
    };
    let over = step >= 3 || !failed.is_empty();

    return <div style={{ display: "flex", flexDirection: "column", height: "100vh", fontFamily: "system-ui, sans-serif", background: "#14161a", color: "#f3f4f6" }}>
        <main style={{ flex: "1", padding: "32px 36px 0", overflow: "auto" }}>{body}</main>
        <footer style={{ display: "flex", justifyContent: "flex-end", gap: "8px", padding: "16px 36px 24px" }}>
            <button style={button} disabled={step == 2 && !over} onClick={move |_| { rustx::setup::quit(); }}>{if over { "Close" } else { "Cancel" }}</button>
            <button style={primary} disabled={step == 2 && !over} onClick={move |_| {
                if over {
                    if rustx::setup::launch().is_ok() { rustx::setup::quit(); }
                } else if step == 0 && has_license {
                    step = 1;
                } else if step < 2 {
                    step = 2;
                    task(async move {
                        let result = rustx::setup::install(|done| { percent = done as u32; });
                        match result {
                            Ok(()) => step = 3,
                            Err(why) => failed = why,
                        }
                    });
                }
            }}>{if over { "Start" } else if step == 0 && has_license { "Next" } else if step == 1 { "Accept and install" } else { "Install" }}</button>
        </footer>
    </div>;
}
