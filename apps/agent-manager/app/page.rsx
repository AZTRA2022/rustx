// AI Agent Manager — the overview.
//
// Deploying an agent here means adding it to `agents`, a plain
// `Vec<Agent>` — no Kubernetes, no process spawned. What is real: a
// background loop feeds each running agent's request history, and
// `recharts::LineChart` — off npm — draws it from data this page
// owns, styled with Tailwind utility classes and shadcn/ui's `Card`,
// `Badge` and `Button` rather than hand-rolled look-alikes. The chat
// at the bottom calls `answer_from`, a stubbed reply function,
// instead of a paid LLM API — the honest version of what the
// showcase copy calls "LLM-Studio/Anthropic/OpenRouter API".

#[derive(rustx::ToValue, Clone)]
struct Point {
    at: f64,
    value: f64,
}

#[derive(Clone)]
struct Agent {
    name: String,
    model: String,
    running: bool,
    requests: u32,
    history: Vec<f64>,
}

let mut agents: Vec<Agent> = vec![
    Agent { name: "scout".to_string(), model: "haiku-class".to_string(), running: true, requests: 0, history: Vec::new() },
    Agent { name: "closer".to_string(), model: "sonnet-class".to_string(), running: true, requests: 0, history: Vec::new() },
    Agent { name: "archivist".to_string(), model: "haiku-class".to_string(), running: false, requests: 0, history: Vec::new() },
];
let mut selected: usize = 0;
let mut chat: Vec<(String, String)> = Vec::new();
let mut draft = String::new();
let mut pending: Option<String> = None;
let mut sending = false;

fn Page() -> Element {
    async move {
        let mut step: u64 = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(700)).await;
            step += 1;
            for agent in agents.iter_mut() {
                if agent.running {
                    let wave = (step as f64 * 0.6 + agent.name.len() as f64).sin().abs();
                    let bump = 2.0 + wave * 6.0;
                    agent.requests += bump as u32;
                    agent.history.push(bump);
                    if agent.history.len() > 24 {
                        agent.history.remove(0);
                    }
                }
            }
        }
    };

    async move {
        let mut handled: Option<String> = None;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            let Some(question) = pending.clone() else { continue; };
            if handled.as_deref() == Some(question.as_str()) {
                continue;
            }
            handled = Some(question.clone());
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let reply = answer_from(&question);
            chat.push(("agent".to_string(), reply));
            sending = false;
            pending = None;
        }
    };

    return <div className="mx-auto max-w-6xl px-6 py-8">
        <h1 className="text-2xl font-semibold tracking-tight">AI Agent Manager</h1>
        <p className="mt-1 text-sm text-muted-foreground">deploy, watch, and talk to a handful of agents.</p>

        <div className="mt-6 grid grid-cols-1 gap-5 lg:grid-cols-[280px_1fr]">
            {AgentList()}
            <div className="flex flex-col gap-5">
                {Chart()}
                {Chat()}
            </div>
        </div>
    </div>;
}

fn AgentList() -> Element {
    return <Card className="border-border bg-card">
        <CardHeader>
            <CardTitle className="text-xs uppercase tracking-wider text-muted-foreground">agents</CardTitle>
        </CardHeader>
        <CardContent>
            <ul id="agents" className="flex flex-col gap-2">
                {agents.iter().enumerate().map(|(index, agent)| AgentRow(index, agent.clone())).collect::<Vec<_>>()}
            </ul>
        </CardContent>
    </Card>;
}

fn AgentRow(index: usize, agent: Agent) -> Element {
    let active = selected == index;
    let name_for_route = agent.name.clone();
    return <li key={index.to_string()} className={format!("rounded-lg border p-2.5 {}", if active { "border-primary bg-accent" } else { "border-transparent" })}>
        <div className="cursor-pointer" onClick={move |_| selected = index}>
            <div className="flex items-center justify-between">
                <span className="font-medium">{agent.name.clone()}</span>
                <span className={format!("h-2 w-2 rounded-full {}", if agent.running { "bg-emerald-400" } else { "bg-zinc-600" })}></span>
            </div>
            <div className="text-xs text-muted-foreground">{agent.model.clone()} · {agent.requests} reqs</div>
        </div>
        <div className="mt-2 flex gap-2">
            <Button variant="outline" className="h-7 px-2 text-xs" onClick={move |_| {
                if let Some(agent) = agents.get_mut(index) {
                    agent.running = !agent.running;
                }
            }}>
                {if agent.running { "stop" } else { "start" }}
            </Button>
            <Button variant="ghost" className="h-7 px-2 text-xs" onClick={move |_| rustx::router::navigate(format!("/agent/{}", name_for_route))}>
                open →
            </Button>
        </div>
    </li>;
}

fn Chart() -> Element {
    let agent = agents.get(selected.min(agents.len().saturating_sub(1)));
    let data: Vec<Point> = match agent {
        Some(agent) => agent.history.iter().enumerate().map(|(index, value)| Point { at: index as f64, value: *value }).collect(),
        None => Vec::new(),
    };
    let name = agent.map(|agent| agent.name.clone()).unwrap_or_default();

    return <Card className="border-border bg-card">
        <CardHeader>
            <CardTitle className="text-xs uppercase tracking-wider text-muted-foreground">{name} — requests over time</CardTitle>
        </CardHeader>
        <CardContent>
            <recharts::LineChart width={620} height={200} data={data}>
                <recharts::XAxis dataKey="at" stroke="#52525b" />
                <recharts::YAxis stroke="#52525b" />
                <recharts::Line type="monotone" dataKey="value" stroke="#818cf8" strokeWidth={2} isAnimationActive={false} />
            </recharts::LineChart>
        </CardContent>
    </Card>;
}

fn Chat() -> Element {
    return <Card className="border-border bg-card">
        <CardHeader>
            <CardTitle className="text-xs uppercase tracking-wider text-muted-foreground">talk to the selected agent</CardTitle>
        </CardHeader>
        <CardContent>
            <div id="chat" className="mb-3 flex max-h-44 flex-col gap-1.5 overflow-auto">
                {chat.iter().enumerate().map(|(index, message)| ChatLine(index, message.clone())).collect::<Vec<_>>()}
                {if sending { <p className="text-xs text-muted-foreground">thinking…</p> } else { <span></span> }}
            </div>
            <div className="flex gap-2">
                <input id="draft" placeholder="ask it something" value={draft.clone()}
                    onInput={move |event| draft = event.value}
                    onKeyDown={move |event| if event.key == "Enter" { send(); }}
                    className="flex-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm" />
                <Button onClick={move |_| send()}>send</Button>
            </div>
        </CardContent>
    </Card>;
}

fn ChatLine(index: usize, message: (String, String)) -> Element {
    let (who, text) = message;
    return <p key={index.to_string()} className="text-sm">
        <b className={if who == "you" { "text-indigo-300" } else { "text-emerald-400" }}>{who}</b>: {text}
    </p>;
}

fn send() {
    if draft.trim().is_empty() || pending.is_some() {
        return;
    }
    let question = draft.trim().to_string();
    chat.push(("you".to_string(), question.clone()));
    pending = Some(question);
    sending = true;
    draft = String::new();
}

/// A stubbed reply, standing in for a real LLM API call this demo does
/// not reach for.
fn answer_from(question: &str) -> String {
    let lower = question.to_lowercase();
    if lower.contains("status") || lower.contains("how") {
        "All agents I can see are reporting their last tick fine.".to_string()
    } else if lower.contains("cost") || lower.contains("price") {
        "This reply is stubbed, so it costs nothing — a real deployment would call the model's API here.".to_string()
    } else {
        format!("Noted: \"{}\". (stub reply — no LLM API is called by this demo)", question.trim())
    }
}
