// `/billing` — what the fleet costs, by month and by agent.
//
// The chart is recharts, fed by this page's own numbers. The one thing
// that leaves the app is real: an invoice is written to disk through
// the system's save dialog and announced with a real notification.

#[derive(rustx::ToValue, Clone)]
struct Month {
    at: String,
    tokens: f64,
    tools: f64,
}

#[derive(Clone)]
struct LineItem {
    agent: String,
    runs: u64,
    cost: f64,
}

let mut bill_months: Vec<Month> = Vec::new();
let mut bill_items: Vec<LineItem> = Vec::new();
let mut bill_note: String = String::new();

fn Page() -> Element {
    effect((), || {
        if bill_months.is_empty() {
            bill_months = starting_months();
            bill_items = starting_items();
        }
    });

    let total: f64 = bill_items.iter().map(|item| item.cost).sum();

    return <div key="billing" id="screen-billing" className="px-8 py-7">
        <div className="text-[13px] text-[#8b8f8b]">Workspace</div>
        <div className="mt-2 flex items-center gap-4">
            <h1 className="text-[30px] font-semibold tracking-tight">Billing</h1>
            <span className="rounded-full bg-[#133021] px-3 py-1 text-[13px] text-[#3ddc84]">
                {format!("${:.2} this month", total)}
            </span>
            <button
                className="ml-auto rounded-xl border border-border bg-[#131413] px-3 py-2 text-[13px] text-[#9aa09a] hover:text-foreground"
                onClick={move |_| write_invoice()}
            >
                Download the invoice
            </button>
        </div>
        <p className="mt-2 text-[12px] text-[#5d615d]">{bill_note.clone()}</p>

        <div className="mt-5 grid grid-cols-[1.4fr_1fr] gap-4">
            <div className="rounded-2xl border border-border bg-[#101110] p-5">
                <h2 className="text-[19px] font-semibold tracking-tight">Spend by month</h2>
                <div className="mt-3 flex items-center gap-5">
                    <span className="flex items-center gap-2 text-[13px] text-[#b6bbb6]">
                        <span className="h-[9px] w-[9px] rounded-[2px] bg-[#6f746f]"></span>
                        Tokens
                    </span>
                    <span className="flex items-center gap-2 text-[13px] text-[#b6bbb6]">
                        <span className="h-[9px] w-[9px] rounded-[2px] bg-[#3ddc84]"></span>
                        Tool calls
                    </span>
                </div>
                <div className="mt-4 rounded-xl bg-[#0c0d0c] p-4">
                    <recharts::BarChart width={560} height={230} data={bill_months.clone()} barCategoryGap="28%">
                        <recharts::CartesianGrid stroke="#1b1d1b" strokeDasharray="2 4" vertical={false} />
                        <recharts::XAxis dataKey="at" stroke="#1b1d1b" tickLine={false} />
                        <recharts::YAxis stroke="#1b1d1b" tickLine={false} width={44} unit="$" />
                        <recharts::Tooltip isAnimationActive={false} cursor={false} />
                        <recharts::Bar dataKey="tokens" name="Tokens" stackId="spend" fill="url(#hatch)" isAnimationActive={false} />
                        <recharts::Bar dataKey="tools" name="Tool calls" stackId="spend" fill="#3ddc84" isAnimationActive={false} />
                    </recharts::BarChart>
                </div>
            </div>

            <div className="rounded-2xl border border-border bg-[#101110] p-5">
                <h2 className="text-[19px] font-semibold tracking-tight">By agent</h2>
                <table id="items" className="mt-4 w-full text-left">
                    <thead>
                        <tr className="text-[13px] text-[#8b8f8b]">
                            <th className="pb-2 font-normal">Agent</th>
                            <th className="pb-2 text-right font-normal">Runs</th>
                            <th className="pb-2 text-right font-normal">Cost</th>
                        </tr>
                    </thead>
                    <tbody>
                        {bill_items.iter().map(|item| ItemRow(item.clone())).collect::<Vec<_>>()}
                    </tbody>
                </table>
                <div className="mt-4 flex items-baseline border-t border-border pt-3">
                    <span className="text-[13px] text-[#8b8f8b]">Total</span>
                    <span className="ml-auto font-mono text-[18px]">{format!("${:.2}", total)}</span>
                </div>
            </div>
        </div>
    </div>;
}

fn ItemRow(item: LineItem) -> Element {
    let slug = item.agent.to_lowercase().replace(' ', "-");
    return <tr key={item.agent.clone()} className="border-t border-border text-[13px]">
        <td className="py-2.5">
            <button className="hover:text-[#3ddc84]" onClick={move |_| rustx::router::navigate(format!("/agent/{slug}"))}>
                {item.agent.clone()}
            </button>
        </td>
        <td className="py-2.5 text-right font-mono text-[#b6bbb6]">{item.runs.to_string()}</td>
        <td className="py-2.5 text-right font-mono text-[#b6bbb6]">{format!("${:.2}", item.cost)}</td>
    </tr>;
}

fn write_invoice() {
    let ask = rustx::dialog::Ask::new("Save the invoice")
        .filter("Plain text", ["txt"])
        .named("vantage-invoice.txt");
    let Ok(path) = rustx::dialog::save(ask) else {
        bill_note = "the invoice was not saved".to_string();
        return;
    };
    let mut text = String::from("Vantage — invoice\n\nagent,runs,cost\n");
    let mut total = 0.0;
    for item in bill_items.iter() {
        text.push_str(&format!("{},{},{:.2}\n", item.agent, item.runs, item.cost));
        total += item.cost;
    }
    text.push_str(&format!("\ntotal,{total:.2}\n"));
    match std::fs::write(&path, text) {
        Ok(()) => {
            bill_note = format!("written to {}", path.display());
            let _ = rustx::notify::show(rustx::notify::What::new(
                "Vantage",
                format!("Invoice saved — ${total:.2}."),
            ));
        }
        Err(error) => bill_note = format!("the invoice failed: {error}"),
    }
}

fn starting_months() -> Vec<Month> {
    let names = ["Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov"];
    names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let wave = ((index as f64 * 0.7).sin() + 1.0) / 2.0;
            Month {
                at: name.to_string(),
                tokens: (180.0 + wave * 420.0).round(),
                tools: (40.0 + wave * 90.0).round(),
            }
        })
        .collect()
}

fn starting_items() -> Vec<LineItem> {
    vec![
        LineItem { agent: "Support Triage".to_string(), runs: 12_480, cost: 318.40 },
        LineItem { agent: "Refund Copilot".to_string(), runs: 3_902, cost: 96.10 },
        LineItem { agent: "Lead Qualifier".to_string(), runs: 8_117, cost: 204.72 },
        LineItem { agent: "Doc Indexer".to_string(), runs: 640, cost: 18.05 },
        LineItem { agent: "Churn Watcher".to_string(), runs: 1_204, cost: 31.88 },
    ]
}
