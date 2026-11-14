// FEEDBACK — the right-hand link in the footer.
//
// Nothing is sent anywhere, and the page says so before you type. The
// note is written to a real file where the system's own dialog puts
// it, which is what a demo on a machine with no network can honestly
// offer.

let mut activity: Vec<Happened> = Vec::new();
let mut note: String = String::new();
let mut mood: String = "It works".to_string();
let mut text: String = String::new();
let mut sent: Vec<(String, String)> = Vec::new();

fn Page() -> Element {
    return <div key="feedback" className="px-5 py-4">
        <h1 className="text-[13px] font-semibold text-foreground">Feedback</h1>
        <p className="mt-0.5 max-w-[560px] text-[11px] leading-[1.7] text-muted">
            This window has no network behind it. What you write here is kept for as long as the
            application is open, and written to a real file if you ask for one — it is not sent.
        </p>

        <div className="mt-4 max-w-[560px] rounded-md border border-border bg-[#0f131a] p-4">
            <div className="text-[9px] tracking-[0.14em] text-faint">HOW IS IT GOING</div>
            <div className="mt-2.5 flex gap-1.5">
                {Mood("It works".to_string())}
                {Mood("Something is off".to_string())}
                {Mood("It is broken".to_string())}
            </div>

            <div className="mt-4 text-[9px] tracking-[0.14em] text-faint">WHAT HAPPENED</div>
            <textarea
                id="feedback-text"
                className="mt-2 h-28 w-full resize-none rounded-md border border-border bg-[#11151b] px-3 py-2 text-[11px] leading-[1.6] text-foreground outline-none placeholder:text-faint"
                placeholder="The department pane did not keep my filter when I changed tabs…"
                value={text.clone()}
                onInput={move |event| text = event.value}
            ></textarea>

            <div className="mt-3 flex gap-2">
                <button
                    className="rounded-md bg-blue px-3 py-[6px] text-[11px] text-white hover:brightness-110 disabled:opacity-50"
                    disabled={text.trim().is_empty()}
                    onClick={move |_| file_it()}
                >
                    File it here
                </button>
                <button
                    className="rounded-md border border-border bg-[#11151b] px-3 py-[6px] text-[11px] text-foreground hover:border-faint disabled:opacity-50"
                    disabled={text.trim().is_empty()}
                    onClick={move |_| save_it()}
                >
                    Write it to a file
                </button>
            </div>
        </div>

        {if sent.is_empty() {
            <span></span>
        } else {
            <div className="mt-3 max-w-[560px] rounded-md border border-border bg-[#0f131a] p-4">
                <div className="text-[9px] tracking-[0.14em] text-faint">FILED IN THIS SESSION</div>
                <div className="mt-3 flex flex-col gap-2.5">
                    {sent.iter().enumerate().map(|(index, filed)| Filed(index, filed.0.clone(), filed.1.clone())).collect::<Vec<_>>()}
                </div>
            </div>
        }}
    </div>;
}

fn Mood(name: String) -> Element {
    let active = mood == name;
    let classes = format!(
        "rounded-md px-2.5 py-[6px] text-[11px] {}",
        if active { "bg-blue text-white" } else { "border border-border bg-[#11151b] text-muted hover:text-foreground" }
    );
    let chosen = name.clone();
    return <button key={name.clone()} className={classes} onClick={move |_| mood = chosen.clone()}>
        {name}
    </button>;
}

fn Filed(index: usize, mood: String, said: String) -> Element {
    return <div key={index.to_string()} className="text-[11px]">
        <div className="text-faint">{mood}</div>
        <div className="mt-0.5 leading-[1.6] text-foreground">{said}</div>
    </div>;
}

fn file_it() {
    let said = text.trim().to_string();
    if said.is_empty() {
        return;
    }
    let mut all = sent.clone();
    all.insert(0, (mood.clone(), said.clone()));
    sent = all;

    let mut happened = activity.clone();
    happened.insert(
        0,
        Happened {
            who: "Dara Whitlock".to_string(),
            what: "filed feedback".to_string(),
            when_said: "just now".to_string(),
            week: false,
        },
    );
    happened.truncate(14);
    activity = happened;

    text = String::new();
    note = "the note is filed in this window".to_string();
}

fn save_it() {
    let said = text.trim().to_string();
    if said.is_empty() {
        return;
    }
    let ask = rustx::dialog::Ask::new("Write this note to a file")
        .filter("Plain text", ["txt"])
        .named("reelhouse-feedback.txt");
    let Ok(path) = rustx::dialog::save(ask) else {
        note = "the note was not written".to_string();
        return;
    };
    match std::fs::write(&path, format!("{}\n\n{said}\n", mood.clone())) {
        Ok(()) => {
            note = format!("the note is at {}", path.display());
            let _ = rustx::os::shell::remember_recent(&path);
            file_it();
        }
        Err(error) => note = format!("the note could not be written: {error}"),
    }
}
