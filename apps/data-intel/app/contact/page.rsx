// CONTACT — what the button in the corner opens.
//
// There is no inbox behind this window and the page says so before
// you type. What it does is assemble the brief and put it somewhere
// you can actually take it from: the real clipboard, or a real file.

let mut tables: Vec<Dataset> = Vec::new();
let mut status: String = String::new();

let mut who: String = String::new();
let mut org: String = String::new();
let mut reach: String = String::new();
let mut about: String = "Platform teams".to_string();
let mut said: String = String::new();

fn Page() -> Element {
    let ready = !who.trim().is_empty() && !reach.trim().is_empty();
    let rows: usize = tables.iter().map(|table| table.rows.len()).sum();

    return <div key="contact" id="screen-contact" className="mx-auto max-w-[1000px] px-6 py-10">
        <div className="text-[10px] tracking-[0.16em] text-faint">CONTACT</div>
        <h1 className="mt-2 text-[22px] font-semibold tracking-tight">Tell us what you are drowning in</h1>
        <p className="mt-2.5 max-w-[600px] text-[12px] leading-[1.8] text-muted-foreground">
            Nothing on this page is sent anywhere — this application has no network behind it, by
            construction. What it writes is a brief you can carry: on the clipboard, or in a file.
        </p>

        <div className="mt-7 grid grid-cols-[1fr_360px] gap-5">
            <div className="rounded-[6px] border border-border bg-card p-6">
                <div className="text-[10px] tracking-[0.16em] text-faint">THE BRIEF</div>

                <input
                    id="who"
                    className="mt-4 w-full rounded-[5px] border border-border bg-[#0d0d0d] px-3 py-[8px] text-[12px] outline-none placeholder:text-faint"
                    placeholder="your name"
                    value={who.clone()}
                    onInput={move |event| who = event.value}
                />
                <input
                    id="org"
                    className="mt-2 w-full rounded-[5px] border border-border bg-[#0d0d0d] px-3 py-[8px] text-[12px] outline-none placeholder:text-faint"
                    placeholder="organisation"
                    value={org.clone()}
                    onInput={move |event| org = event.value}
                />
                <input
                    id="reach"
                    className="mt-2 w-full rounded-[5px] border border-border bg-[#0d0d0d] px-3 py-[8px] text-[12px] outline-none placeholder:text-faint"
                    placeholder="where to reach you"
                    value={reach.clone()}
                    onInput={move |event| reach = event.value}
                />

                <div className="mt-3 flex flex-wrap gap-1.5">
                    {rooms().iter().map(|room| Pick(room.clone())).collect::<Vec<_>>()}
                </div>

                <textarea
                    id="said"
                    className="mt-3 h-28 w-full resize-none rounded-[5px] border border-border bg-[#0d0d0d] px-3 py-[8px] text-[12px] leading-[1.6] outline-none placeholder:text-faint"
                    placeholder="how many rows a day, and which three of them you would actually act on"
                    value={said.clone()}
                    onInput={move |event| said = event.value}
                ></textarea>

                <div className="mt-4 flex gap-2">
                    <button className="slab rounded-[5px] px-4 py-[8px] text-[11px]" disabled={!ready} onClick={move |_| copy_brief()}>
                        COPY THE BRIEF
                    </button>
                    <button className="glossy rounded-[5px] px-4 py-[8px] text-[11px]" disabled={!ready} onClick={move |_| save_brief()}>
                        SAVE IT TO A FILE
                    </button>
                </div>
                <p className="mt-3 text-[11px] text-muted-foreground">
                    {if ready { status.clone() } else { "A name and a way to reach you are both needed.".to_string() }}
                </p>
            </div>

            <div>
                <div className="rounded-[6px] border border-border bg-card p-5">
                    <div className="text-[10px] tracking-[0.16em] text-faint">WHAT THE BRIEF WILL SAY</div>
                    <pre id="brief" className="mt-3 whitespace-pre-wrap break-words text-[10px] leading-[1.8] text-muted-foreground">
                        {brief()}
                    </pre>
                </div>
                <div className="mt-4 rounded-[6px] border border-border bg-card p-5">
                    <div className="text-[10px] tracking-[0.16em] text-faint">BEFORE YOU WRITE</div>
                    <div className="mt-2.5 flex flex-col gap-1">
                        {Link("See the console first".to_string(), "/console".to_string())}
                        {Link("What is real and what is described".to_string(), "/platform".to_string())}
                        {Link("How this is put together".to_string(), "/docs/architecture".to_string())}
                    </div>
                    <p className="mt-3 text-[10px] text-faint">
                        {format!("{} table(s) and {rows} row(s) are open in this window.", tables.len())}
                    </p>
                </div>
            </div>
        </div>
    </div>;
}

fn Pick(name: String) -> Element {
    let here = about == name;
    let classes = format!(
        "rounded-[4px] border px-2.5 py-[4px] text-[10px] {}",
        if here { "border-[#d98a4a] text-warm" } else { "border-border text-faint hover:text-foreground" }
    );
    let chosen = name.clone();
    return <button key={name.clone()} className={classes} onClick={move |_| about = chosen.clone()}>
        {name}
    </button>;
}

fn Link(title: String, route: String) -> Element {
    let going = route.clone();
    return <button
        key={route.clone()}
        className="rounded-[4px] px-2 py-[5px] text-left text-[11px] text-muted-foreground hover:bg-[#141414] hover:text-foreground"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        {title}
    </button>;
}

fn rooms() -> Vec<String> {
    vec![
        "Platform teams".to_string(),
        "Finance".to_string(),
        "Growth".to_string(),
        "Anyone with a file".to_string(),
    ]
}

fn brief() -> String {
    format!(
        "R INTELLIGENT DATA — BRIEF\nname: {}\norganisation: {}\nreach: {}\nroom: {}\ntables open: {}\n\n{}\n",
        if who.trim().is_empty() { "—".to_string() } else { who.clone() },
        if org.trim().is_empty() { "—".to_string() } else { org.clone() },
        if reach.trim().is_empty() { "—".to_string() } else { reach.clone() },
        about.clone(),
        tables.len(),
        if said.trim().is_empty() { "(nothing written yet)".to_string() } else { said.clone() }
    )
}

fn copy_brief() {
    match rustx::clipboard::set_text(&brief()) {
        Ok(()) => status = "the brief is on the clipboard".to_string(),
        Err(error) => status = format!("the clipboard refused: {error}"),
    }
}

fn save_brief() {
    let ask = rustx::dialog::Ask::new("Save the brief")
        .filter("Plain text", ["txt"])
        .named("r-intelligent-data-brief.txt");
    let Ok(path) = rustx::dialog::save(ask) else {
        status = "the brief was not saved".to_string();
        return;
    };
    match std::fs::write(&path, brief()) {
        Ok(()) => {
            status = format!("the brief is at {}", path.display());
            let _ = rustx::os::shell::remember_recent(&path);
        }
        Err(error) => status = format!("the brief could not be written: {error}"),
    }
}
