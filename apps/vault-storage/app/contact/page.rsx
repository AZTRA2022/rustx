// CONTACT — what "Let's Talk" opens.
//
// There is no inbox behind this window and the page says so before
// you type. What it does instead is assemble the brief a custody
// officer would ask for and put it on the real clipboard, or write it
// to a real file through the system's own save dialog.

let mut items: Vec<VaultItem> = Vec::new();
let mut said: String = String::new();

let mut who: String = String::new();
let mut org: String = String::new();
let mut reach: String = String::new();
let mut plan: String = "CUSTODY".to_string();
let mut need: String = String::new();

fn Page() -> Element {
    let ready = !who.trim().is_empty() && !reach.trim().is_empty();

    return <div key="contact" id="screen-contact" className="plain">
        <div className="label">CONTACT</div>
        <h1 className="h1" style={{ marginTop: "8px" }}>Let's talk</h1>
        <p className="lead" style={{ marginTop: "10px", maxWidth: "600px" }}>
            Nothing on this page is sent anywhere — this application has no network behind it, by
            construction. What it writes is a brief you can carry: on the clipboard, or in a file.
        </p>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 330px", gap: "26px", marginTop: "24px", alignItems: "start" }}>
            <div className="panel">
                <div className="label">THE BRIEF</div>

                <input
                    id="who"
                    className="field-input"
                    style={{ marginTop: "12px" }}
                    placeholder="your name"
                    value={who.clone()}
                    onInput={move |event| who = event.value}
                />
                <input
                    id="org"
                    className="field-input"
                    style={{ marginTop: "8px" }}
                    placeholder="organisation"
                    value={org.clone()}
                    onInput={move |event| org = event.value}
                />
                <input
                    id="reach"
                    className="field-input"
                    style={{ marginTop: "8px" }}
                    placeholder="where to reach you"
                    value={reach.clone()}
                    onInput={move |event| reach = event.value}
                />

                <div className="row" style={{ marginTop: "12px", gap: "7px" }}>
                    {plans().iter().map(|one| Pick(one.name.clone())).collect::<Vec<_>>()}
                </div>

                <textarea
                    id="need"
                    className="field-input"
                    style={{ marginTop: "12px", height: "96px", resize: "none" }}
                    placeholder="what you are holding, and what would have to be true before you held it here"
                    value={need.clone()}
                    onInput={move |event| need = event.value}
                ></textarea>

                <div className="row" style={{ marginTop: "14px" }}>
                    <button className="cta" disabled={!ready} onClick={move |_| copy_brief()}>
                        COPY THE BRIEF
                    </button>
                    <button className="ghost" disabled={!ready} onClick={move |_| save_brief()}>
                        SAVE IT TO A FILE
                    </button>
                </div>
                <p className="readout" style={{ marginTop: "10px" }}>
                    {if ready {
                        said.clone()
                    } else {
                        "A name and a way to reach you are both needed.".to_string()
                    }}
                </p>
            </div>

            <div>
                <div className="panel-dark">
                    <div style={{ fontSize: "10px", letterSpacing: "0.1em" }}>[WHAT THE BRIEF WILL SAY]</div>
                    <pre id="brief" style={{ fontSize: "10px", lineHeight: "1.75", color: "#c9c9c9", margin: "12px 0 0", whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
                        {brief()}
                    </pre>
                </div>

                <div className="panel" style={{ marginTop: "14px" }}>
                    <div className="label">BEFORE YOU WRITE</div>
                    <div style={{ marginTop: "9px", display: "flex", flexDirection: "column", gap: "3px" }}>
                        {Link("What the cipher actually is".to_string(), "/security".to_string())}
                        {Link("The three plans".to_string(), "/pricing".to_string())}
                        {Link("Quickstart".to_string(), "/docs/quickstart".to_string())}
                        {Link("Try the vault first".to_string(), "/vault".to_string())}
                    </div>
                </div>
            </div>
        </div>
    </div>;
}

fn Pick(name: String) -> Element {
    let here = plan == name;
    let classes = if here { "chip on".to_string() } else { "chip".to_string() };
    let chosen = name.clone();
    return <button key={name.clone()} className={classes} style={{ background: "none" }} onClick={move |_| plan = chosen.clone()}>
        {name}
    </button>;
}

fn Link(title: String, route: String) -> Element {
    let going = route.clone();
    return <button
        key={route.clone()}
        style={{ background: "none", border: "none", padding: "5px 0", textAlign: "left", fontSize: "12px", color: "#767676" }}
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        {title}
    </button>;
}

fn brief() -> String {
    format!(
        "CRYOSTASIS — CUSTODY BRIEF\nname: {}\norganisation: {}\nreach: {}\nplan: {}\nvault in this window: {} item(s)\n\n{}\n",
        if who.trim().is_empty() { "—".to_string() } else { who.clone() },
        if org.trim().is_empty() { "—".to_string() } else { org.clone() },
        if reach.trim().is_empty() { "—".to_string() } else { reach.clone() },
        plan.clone(),
        items.len(),
        if need.trim().is_empty() { "(nothing written yet)".to_string() } else { need.clone() }
    )
}

fn copy_brief() {
    match rustx::clipboard::set_text(&brief()) {
        Ok(()) => said = "the brief is on the clipboard".to_string(),
        Err(error) => said = format!("the clipboard refused: {error}"),
    }
}

fn save_brief() {
    let ask = rustx::dialog::Ask::new("Save the custody brief")
        .filter("Plain text", ["txt"])
        .named("cryostasis-brief.txt");
    let Ok(path) = rustx::dialog::save(ask) else {
        said = "the brief was not saved".to_string();
        return;
    };
    match std::fs::write(&path, brief()) {
        Ok(()) => {
            said = format!("the brief is at {}", path.display());
            let _ = rustx::os::shell::remember_recent(&path);
        }
        Err(error) => said = format!("the brief could not be written: {error}"),
    }
}
