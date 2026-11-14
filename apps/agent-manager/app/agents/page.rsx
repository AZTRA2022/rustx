// `/agents` — what the sidebar's "Agents" item points at: the fleet as
// cards, each one a way into `/agent/<slug>`.
//
// The fleet is declared with the same names the layout declares it
// with, which is how two files of one application look at one list: a
// page's values are kept by name for the whole application, so
// `agent_fleet` here and `agent_fleet` there are the same cell.

#[derive(Clone)]
struct Card {
    name: String,
    slug: String,
    model: String,
    runs: u64,
    rate: f64,
    live: bool,
}

let mut agents_list: Vec<Card> = Vec::new();
let mut agents_query: String = String::new();

fn Page() -> Element {
    effect((), || {
        if agents_list.is_empty() {
            agents_list = starting_fleet();
        }
    });

    let needle = agents_query.to_lowercase();
    let shown: Vec<Card> = agents_list
        .iter()
        .filter(|card| needle.is_empty() || card.name.to_lowercase().contains(&needle))
        .cloned()
        .collect();

    return <div key="agents" id="screen-agents" className="px-8 py-7">
        <div className="text-[13px] text-[#8b8f8b]">Workspace</div>
        <div className="mt-2 flex items-center gap-4">
            <h1 className="text-[30px] font-semibold tracking-tight">Agents</h1>
            <span className="rounded-full bg-[#1c1e1c] px-3 py-1 text-[13px] text-[#8b8f8b]">
                {format!("{} deployed", agents_list.len())}
            </span>
            <input
                id="filter"
                className="ml-auto w-[260px] rounded-xl border border-border bg-[#131413] px-3 py-2 text-[13px] outline-none placeholder:text-[#5d615d]"
                placeholder="Filter the fleet"
                value={agents_query.clone()}
                onInput={move |event| agents_query = event.value}
            />
            <button
                className="rounded-xl border border-border bg-[#131413] px-3 py-2 text-[13px] text-[#9aa09a] hover:text-foreground"
                onClick={move |_| hire()}
            >
                Deploy an agent
            </button>
        </div>

        <div id="cards" className="mt-6 grid grid-cols-3 gap-4">
            {shown.iter().map(|card| AgentCard(card.clone())).collect::<Vec<_>>()}
        </div>
    </div>;
}

fn AgentCard(card: Card) -> Element {
    let route = format!("/agent/{}", card.slug);
    let bar = format!(
        "height:4px;border-radius:2px;background:linear-gradient(90deg,#3ddc84 {}%,#1c1e1c {}%)",
        card.rate, card.rate
    );
    return <button
        key={card.slug.clone()}
        className="rounded-2xl border border-border bg-[#101110] p-5 text-left hover:border-[#2d312d]"
        onClick={move |_| rustx::router::navigate(route.clone())}
    >
        <div className="flex items-center gap-2.5">
            {if card.live {
                <span className="flex h-4 w-4 items-center justify-center rounded-full border border-[#3ddc84]">
                    <span className="h-[6px] w-[6px] rounded-full bg-[#3ddc84]"></span>
                </span>
            } else {
                <span className="h-4 w-4 rounded-full border border-dashed border-[#4a4f4a]"></span>
            }}
            <span className="text-[16px] font-medium">{card.name.clone()}</span>
            <span className="ml-auto text-[12px] text-[#5d615d]">{card.model.clone()}</span>
        </div>
        <div className="mt-4 flex items-baseline gap-2">
            <span className="font-mono text-[24px]">{thousands(card.runs)}</span>
            <span className="text-[12px] text-[#8b8f8b]">runs</span>
            <span className="ml-auto font-mono text-[13px] text-[#3ddc84]">{format!("{:.1}%", card.rate)}</span>
        </div>
        <div className="mt-3" style={bar}></div>
    </button>;
}

fn hire() {
    let mut fleet = agents_list.clone();
    let number = fleet.len() + 1;
    fleet.push(Card {
        name: format!("New Agent {number}"),
        slug: format!("new-agent-{number}"),
        model: "haiku-class".to_string(),
        runs: 0,
        rate: 100.0,
        live: false,
    });
    agents_list = fleet;
}

fn starting_fleet() -> Vec<Card> {
    vec![
        Card { name: "Support Triage".to_string(), slug: "support-triage".to_string(), model: "sonnet-class".to_string(), runs: 12_480, rate: 97.4, live: true },
        Card { name: "Refund Copilot".to_string(), slug: "refund-copilot".to_string(), model: "haiku-class".to_string(), runs: 3_902, rate: 99.1, live: false },
        Card { name: "Lead Qualifier".to_string(), slug: "lead-qualifier".to_string(), model: "sonnet-class".to_string(), runs: 8_117, rate: 95.8, live: true },
        Card { name: "Doc Indexer".to_string(), slug: "doc-indexer".to_string(), model: "haiku-class".to_string(), runs: 640, rate: 100.0, live: false },
        Card { name: "Churn Watcher".to_string(), slug: "churn-watcher".to_string(), model: "haiku-class".to_string(), runs: 1_204, rate: 92.3, live: false },
    ]
}

fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}
