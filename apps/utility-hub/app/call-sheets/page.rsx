// CALL SHEETS — the schedule. Each day is a card and a route:
// `/call-sheets/<day>` is the sheet itself.

let mut note: String = String::new();
let mut only_open: bool = false;

fn Page() -> Element {
    let sheets: Vec<CallSheet> = call_sheets()
        .into_iter()
        .filter(|sheet| !only_open || !sheet.locked)
        .collect();

    return <div key="call-sheets" className="px-5 py-4">
        <div className="flex items-start">
            <div>
                <h1 className="text-[13px] font-semibold text-foreground">Call Sheets</h1>
                <p className="mt-0.5 text-[11px] text-muted">The shooting schedule, one day at a time.</p>
            </div>
            <button
                className="ml-auto rounded-md border border-border bg-[#11151b] px-2.5 py-[6px] text-[11px] text-foreground hover:border-faint"
                onClick={move |_| only_open = !only_open}
            >
                {if only_open { "Showing open days" } else { "Showing every day" }}
            </button>
        </div>

        <div id="sheets" className="mt-4 flex flex-col gap-2">
            {sheets.iter().map(|sheet| Card(sheet.clone())).collect::<Vec<_>>()}
        </div>

        {if sheets.is_empty() {
            <p className="mt-4 text-[11px] text-faint">Every day on the schedule is locked.</p>
        } else {
            <span></span>
        }}
    </div>;
}

fn Card(sheet: CallSheet) -> Element {
    let going = format!("/call-sheets/{}", sheet.slug);
    let to_copy = sheet.clone();

    return <div key={sheet.slug.clone()} className="rounded-md border border-border bg-[#0f131a] p-3.5">
        <div className="flex items-center gap-2">
            <button className="flex items-center gap-2 text-left" onClick={move |_| rustx::router::navigate(going.clone())}>
                <Folder size={13} color="#6ea0ff" />
                <span className="text-[12px] font-semibold text-foreground">{sheet.day.clone()}</span>
                <span className="text-[10px] text-faint">{sheet.date.clone()}</span>
            </button>
            {if sheet.locked {
                <span className="rounded-[3px] bg-[#1a1f27] px-1.5 py-[1px] text-[9px] text-[#9aa5b1]">LOCKED</span>
            } else {
                <span className="rounded-[3px] bg-[#11291d] px-1.5 py-[1px] text-[9px] text-[#4ec98a]">OPEN</span>
            }}
            <span className="ml-auto text-[10px] tabular-nums text-muted">
                {format!("{} → {}", sheet.call, sheet.wrap)}
            </span>
            <button
                className="rounded-md border border-border bg-[#11151b] px-2 py-[4px] text-[10px] text-muted hover:text-foreground"
                onClick={move |_| copy_sheet(to_copy.clone())}
            >
                copy
            </button>
        </div>
        <div className="mt-2 text-[11px] text-muted">{format!("{} · {}", sheet.unit, sheet.location)}</div>
        <div className="mt-2 flex flex-wrap gap-1.5">
            {sheet.scenes.iter().map(|scene| Scene(sheet.slug.clone(), scene.clone())).collect::<Vec<_>>()}
        </div>
    </div>;
}

fn Scene(slug: String, scene: String) -> Element {
    return <span key={format!("{slug}-{scene}")} className="rounded-[4px] border border-[#243040] px-2 py-[3px] text-[10px] text-foreground">
        {scene}
    </span>;
}

/// The whole sheet as plain text, on the real clipboard — which is
/// what a 1st AD actually does with one.
fn copy_sheet(sheet: CallSheet) {
    let text = format!(
        "{} ({})\n{}\n{}\ncall {} · wrap {}\n{}",
        sheet.day,
        sheet.date,
        sheet.unit,
        sheet.location,
        sheet.call,
        sheet.wrap,
        sheet.scenes.join("\n")
    );
    match rustx::clipboard::set_text(&text) {
        Ok(()) => note = format!("{} is on the clipboard", sheet.day),
        Err(error) => note = format!("the clipboard refused: {error}"),
    }
}
