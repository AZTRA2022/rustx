// BUDGET — what each head of the production was given and what it has
// spent.
//
// The lines are the production's own. The totals, the overspend
// warnings and the department filter are computed from them as the
// page draws, and a line can be adjusted here: typing a new figure
// changes the totals under it.

let mut note: String = String::new();
let mut lines: Vec<BudgetLine> = Vec::new();
let mut head_filter: String = "All".to_string();

fn Page() -> Element {
    effect((), || {
        if lines.is_empty() {
            lines = budget_lines();
        }
    });

    let shown: Vec<BudgetLine> = lines
        .iter()
        .filter(|line| head_filter == "All" || line.department == head_filter)
        .cloned()
        .collect();
    let planned: f64 = shown.iter().map(|line| line.planned).sum();
    let spent: f64 = shown.iter().map(|line| line.spent).sum();
    let over = shown.iter().filter(|line| line.spent > line.planned).count();

    let mut heads: Vec<String> = vec!["All".to_string()];
    for line in lines.iter() {
        if !heads.contains(&line.department) {
            heads.push(line.department.clone());
        }
    }

    return <div key="budget" className="px-5 py-4">
        <div className="flex items-start">
            <div>
                <h1 className="text-[13px] font-semibold text-foreground">Budget</h1>
                <p className="mt-0.5 text-[11px] text-muted">
                    {format!("{} of {} spent · {over} line(s) over.", money(spent), money(planned))}
                </p>
            </div>
            <div className="ml-auto flex flex-wrap items-center justify-end gap-1.5">
                {heads.iter().map(|head| Pick(head.clone())).collect::<Vec<_>>()}
            </div>
        </div>

        <div className="mt-4 grid grid-cols-3 gap-2.5">
            {Tile("PLANNED".to_string(), money(planned), "across the lines shown".to_string())}
            {Tile("SPENT".to_string(), money(spent), format!("{:.0}% of the plan", if planned == 0.0 { 0.0 } else { spent * 100.0 / planned }))}
            {Tile("LEFT".to_string(), money(planned - spent), if planned - spent < 0.0 { "the plan is exceeded".to_string() } else { "still to commit".to_string() })}
        </div>

        <div id="budget-lines" className="mt-4 overflow-hidden rounded-md border border-border bg-[#0f131a]">
            <div className="grid grid-cols-[1fr_130px_110px_110px_90px] border-b border-border px-3.5 py-2 text-[10px] tracking-[0.1em] text-faint">
                <span>HEAD</span>
                <span>DEPARTMENT</span>
                <span className="text-right">PLANNED</span>
                <span className="text-right">SPENT</span>
                <span className="text-right">USED</span>
            </div>
            {shown.iter().map(|line| Row(line.clone())).collect::<Vec<_>>()}
        </div>

        <p className="mt-3 text-[10px] text-faint">
            A spent figure is editable: the totals above are recomputed from what is in the fields.
        </p>
    </div>;
}

fn Pick(head: String) -> Element {
    let active = head_filter == head;
    let classes = format!(
        "rounded-md px-2.5 py-[5px] text-[11px] {}",
        if active { "bg-blue text-white" } else { "border border-border bg-[#11151b] text-muted hover:text-foreground" }
    );
    let chosen = head.clone();
    return <button key={head.clone()} className={classes} onClick={move |_| head_filter = chosen.clone()}>
        {head}
    </button>;
}

fn Tile(name: String, value: String, under: String) -> Element {
    return <div key={name.clone()} className="rounded-md border border-border bg-[#0f131a] px-3.5 py-3">
        <div className="text-[9px] tracking-[0.12em] text-faint">{name}</div>
        <div className="mt-1.5 text-[18px] font-semibold tabular-nums text-foreground">{value}</div>
        <div className="mt-0.5 text-[10px] text-muted">{under}</div>
    </div>;
}

fn Row(line: BudgetLine) -> Element {
    let used = if line.planned == 0.0 { 0.0 } else { line.spent * 100.0 / line.planned };
    let over = line.spent > line.planned;
    let ink = if over { "#f0896b" } else { "#4ec98a" };
    let bar = format!("height:4px;border-radius:2px;background-color:{ink};width:{}%", used.clamp(0.0, 100.0));
    let head = line.head.clone();

    return <div key={line.head.clone()} className="grid grid-cols-[1fr_130px_110px_110px_90px] items-center border-b border-border px-3.5 py-2.5 text-[11px] last:border-b-0">
        <span>
            <span className="block text-foreground">{line.head.clone()}</span>
            <span className="mt-1 block h-[4px] w-[180px] rounded-[2px] bg-[#151b24]">
                <span className="block" style={bar}></span>
            </span>
        </span>
        <span className="text-muted">{line.department.clone()}</span>
        <span className="text-right tabular-nums text-muted">{money(line.planned)}</span>
        <span className="text-right">
            <input
                className="w-[92px] rounded-[4px] border border-border bg-[#11151b] px-2 py-[3px] text-right text-[11px] tabular-nums text-foreground outline-none"
                value={format!("{:.0}", line.spent)}
                onInput={move |event| set_spent(head.clone(), event.value)}
            />
        </span>
        <span className="text-right tabular-nums" style={format!("color:{ink}")}>{format!("{:.0}%", used)}</span>
    </div>;
}

fn set_spent(head: String, value: String) {
    let digits: String = value.chars().filter(|letter| letter.is_ascii_digit() || *letter == '.').collect();
    let amount = digits.parse::<f64>().unwrap_or(0.0);
    let mut all = lines.clone();
    if let Some(line) = all.iter_mut().find(|line| line.head == head) {
        line.spent = amount;
    }
    lines = all;
    note = format!("{head} is now at {}", money(amount));
}

/// A sum in the production's own currency.
fn money(amount: f64) -> String {
    if amount.abs() >= 1_000_000.0 {
        return format!("{:.2}M", amount / 1_000_000.0);
    }
    format!("{:.0}k", amount / 1_000.0)
}
