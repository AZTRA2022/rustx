// `/evals` — the suites each agent is graded against.
//
// Running a suite is a real piece of work with a real duration: the
// button starts a background task that walks the cases one at a time
// and writes the score back as it goes, so the bar fills while the
// screen stays answerable.

#[derive(Clone)]
struct Suite {
    name: String,
    agent: String,
    cases: u32,
    /// How many cases have been walked, and how many of those passed.
    done: u32,
    passed: u32,
    running: bool,
}

let mut evals_suites: Vec<Suite> = Vec::new();
let mut evals_note: String = String::new();
let mut evals_wanted: Option<usize> = None;

fn Page() -> Element {
    effect((), || {
        if evals_suites.is_empty() {
            evals_suites = starting_suites();
        }
    });

    // One runner for the whole page: it picks up whatever the buttons
    // asked for and walks that suite's cases, one every 120 ms.
    async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            let Some(index) = evals_wanted else { continue };

            let mut suites = evals_suites.clone();
            let Some(suite) = suites.get_mut(index) else {
                evals_wanted = None;
                continue;
            };
            if !suite.running {
                suite.running = true;
                suite.done = 0;
                suite.passed = 0;
            }

            // A case passes unless its number says otherwise, which is
            // what gives each suite a score of its own.
            let fails = (suite.done as usize + suite.name.len()) % 11 == 0;
            suite.done += 1;
            if !fails {
                suite.passed += 1;
            }

            if suite.done >= suite.cases {
                suite.running = false;
                let said = format!("{} finished — {}/{}", suite.name, suite.passed, suite.cases);
                evals_suites = suites;
                evals_wanted = None;
                evals_note = said.clone();
                let _ = rustx::notify::show(rustx::notify::What::new("Vantage", said));
                continue;
            }
            evals_suites = suites;
        }
    };

    return <div key="evals" id="screen-evals" className="px-8 py-7">
        <div className="text-[13px] text-[#8b8f8b]">Workspace</div>
        <div className="mt-2 flex items-center gap-4">
            <h1 className="text-[30px] font-semibold tracking-tight">Evals</h1>
            <span className="rounded-full bg-[#1c1e1c] px-3 py-1 text-[13px] text-[#8b8f8b]">
                {format!("{} suites", evals_suites.len())}
            </span>
            <button
                className="ml-auto rounded-xl border border-border bg-[#131413] px-3 py-2 text-[13px] text-[#9aa09a] hover:text-foreground"
                onClick={move |_| evals_wanted = Some(0)}
            >
                Run the first suite
            </button>
        </div>
        <p className="mt-2 text-[12px] text-[#5d615d]">{evals_note.clone()}</p>

        <div id="suites" className="mt-5 flex flex-col gap-3">
            {evals_suites.iter().enumerate().map(|(index, suite)| SuiteRow(index, suite.clone())).collect::<Vec<_>>()}
        </div>
    </div>;
}

fn SuiteRow(index: usize, suite: Suite) -> Element {
    let percent = if suite.cases == 0 { 0.0 } else { suite.done as f64 * 100.0 / suite.cases as f64 };
    let bar = format!(
        "height:6px;border-radius:3px;background:linear-gradient(90deg,#3ddc84 {percent}%,#1c1e1c {percent}%)"
    );
    let slug = suite.agent.to_lowercase().replace(' ', "-");
    return <div key={suite.name.clone()} className="rounded-2xl border border-border bg-[#101110] p-5">
        <div className="flex items-center gap-3">
            <span className="text-[16px] font-medium">{suite.name.clone()}</span>
            <button
                className="rounded-full bg-[#1c1e1c] px-3 py-0.5 text-[12px] text-[#8b8f8b] hover:text-[#3ddc84]"
                onClick={move |_| rustx::router::navigate(format!("/agent/{slug}"))}
            >
                {suite.agent.clone()}
            </button>
            <span className="ml-auto font-mono text-[13px] text-[#b6bbb6]">
                {format!("{}/{} passed", suite.passed, suite.done)}
            </span>
            <button
                className="rounded-xl border border-border bg-[#131413] px-3 py-1.5 text-[12px] text-[#9aa09a] hover:text-foreground"
                onClick={move |_| evals_wanted = Some(index)}
            >
                {if suite.running { "running…" } else { "Run" }}
            </button>
        </div>
        <div className="mt-3" style={bar}></div>
    </div>;
}

fn starting_suites() -> Vec<Suite> {
    vec![
        Suite { name: "Refund policy".to_string(), agent: "Support Triage".to_string(), cases: 240, done: 240, passed: 228, running: false },
        Suite { name: "Tone and escalation".to_string(), agent: "Support Triage".to_string(), cases: 120, done: 120, passed: 117, running: false },
        Suite { name: "Charge disambiguation".to_string(), agent: "Refund Copilot".to_string(), cases: 90, done: 90, passed: 86, running: false },
        Suite { name: "Qualification rubric".to_string(), agent: "Lead Qualifier".to_string(), cases: 160, done: 160, passed: 142, running: false },
        Suite { name: "Index freshness".to_string(), agent: "Doc Indexer".to_string(), cases: 60, done: 60, passed: 60, running: false },
    ]
}
