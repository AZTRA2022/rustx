//! Ops Monitor — the service console of "FORGE ORG".
//!
//! Everything with a number on a screen of this app is read from the
//! machine it is running on, not mocked: CPU, core count and
//! per-process load from `sysinfo`; resident and total memory from the
//! same; the graphics adapter's own name, VRAM and live utilisation
//! counter from the platform's shell through `rustx::terminal::run`;
//! the network link from `rustx::os::net`.
//!
//! The marketing copy behind this showcase promises alerts on email and
//! WhatsApp. This app raises a real desktop notification instead and
//! says so, rather than pretending a message left the machine. The
//! service identity — the name "runner-gateway", its description, its
//! deploy history — is this app's own copy, because there is no control
//! plane behind a local demo.
//!
//! The two types below live here rather than in a page because the
//! pages share values of them: a value a page declares is kept by its
//! name for the whole application, and a type declared inside one page
//! module would be a different type in the next one.

use rustx::{App, Config, ToValue};

/// One second of the machine, as the charts read it.
///
/// Not `cpu` / `gpu` for the field names: a struct field whose name is
/// also the name of a value of a page is rewritten by the compiler into
/// a read of that value, which is not valid Rust in field position.
#[derive(ToValue, Clone)]
pub struct Sample {
    pub at: String,
    pub cpu_pct: f64,
    pub gpu_pct: f64,
    pub mem_pct: f64,
}

/// A rule the console watches, and whether it is armed.
#[derive(Clone)]
pub struct Alert {
    pub title: String,
    pub rule: String,
    pub metric: String,
    pub over: f64,
    pub critical: bool,
    pub on: bool,
}

/// One service of the organisation.
#[derive(Clone)]
pub struct Service {
    pub slug: String,
    pub name: String,
    pub kind: String,
    pub summary: String,
    pub healthy: bool,
}

/// The services this console knows about. The first one is the machine
/// this app runs on; the rest are its neighbours, named but not read.
pub fn services() -> Vec<Service> {
    vec![
        Service {
            slug: "runner-gateway".to_string(),
            name: "runner-gateway".to_string(),
            kind: "API · EDGE".to_string(),
            summary: "Edge gateway that queues agent tool calls, streams tokens back to clients and retries failed runs across three regions.".to_string(),
            healthy: true,
        },
        Service {
            slug: "token-router".to_string(),
            name: "token-router".to_string(),
            kind: "API".to_string(),
            summary: "Routes completions between model providers and keeps the per-tenant budget.".to_string(),
            healthy: true,
        },
        Service {
            slug: "eval-worker".to_string(),
            name: "eval-worker".to_string(),
            kind: "BATCH".to_string(),
            summary: "Walks eval suites nightly and files the scores.".to_string(),
            healthy: false,
        },
        Service {
            slug: "doc-indexer".to_string(),
            name: "doc-indexer".to_string(),
            kind: "BATCH".to_string(),
            summary: "Keeps the retrieval index fresh against the docs bucket.".to_string(),
            healthy: true,
        },
    ]
}

/// One topic of the documentation. Two pages share it — the index and
/// the `[slug]` page — so it lives here with the services.
#[derive(Clone)]
pub struct Topic {
    pub slug: String,
    pub title: String,
    pub blurb: String,
    pub body: Vec<(String, String)>,
}

/// Everything the documentation says, written against what the
/// application actually does.
pub fn topics() -> Vec<Topic> {
    vec![
        Topic {
            slug: "reading-the-machine".to_string(),
            title: "Reading the machine".to_string(),
            blurb: "Where every number in this console comes from.".to_string(),
            body: vec![
                (
                    "THE LOOP".to_string(),
                    "A task in the application's root layout refreshes `sysinfo` once a second: the global processor usage, the core count, resident and total memory, and every process on the machine. It keeps the eight heaviest by processor and appends one sample to a history of at most 480 seconds. Because the loop lives in the layout and not in a page, moving between RESOURCE and ANALYTICS does not restart it.".to_string(),
                ),
                (
                    "THE GRAPHICS ADAPTER".to_string(),
                    "The adapter is not in `sysinfo`. A second task asks the platform's own shell through `rustx::terminal::run` — once for the adapter's name and its video memory, then every eight seconds for the utilisation counter. The call blocks, so it runs on a blocking thread of the runtime. When the shell has nothing to say, the GPU card says 'unreadable' rather than showing a zero.".to_string(),
                ),
                (
                    "THE NETWORK".to_string(),
                    "`rustx::os::net::network()` reports the machine's link, which is the pill beside the service name and the LINK tile on Home and Status.".to_string(),
                ),
            ],
        },
        Topic {
            slug: "alerts".to_string(),
            title: "Alerts".to_string(),
            blurb: "What an armed rule does, and what it cannot do.".to_string(),
            body: vec![
                (
                    "WHAT A RULE IS".to_string(),
                    "A rule names a metric — processor, adapter or memory — and a line. The reading loop checks every armed rule on each pass and fires the moment the reading crosses, not on every second it stays over: one notification per crossing.".to_string(),
                ),
                (
                    "WHERE IT GOES".to_string(),
                    "A fired rule raises a real desktop notification through `rustx::notify` and puts a count on the taskbar with `rustx::os::attention::badge`. The marketing copy behind this console promises email and WhatsApp; a local demo cannot send either, so it raises what it can and names it rather than pretending a message left the machine.".to_string(),
                ),
                (
                    "ARMING ONE".to_string(),
                    "The toggles live on the RESOURCE tab. ADD ALERT builds a rule ten points above the current processor reading, so it is a line that can actually be crossed instead of a decoration.".to_string(),
                ),
            ],
        },
        Topic {
            slug: "deploys".to_string(),
            title: "Deploys".to_string(),
            blurb: "What NEW DEPLOY and LIVE LOGS really run.".to_string(),
            body: vec![
                (
                    "NEW DEPLOY".to_string(),
                    "A real script, handed to the shell `rustx::terminal` found on this machine, waited for. The console shows the exit code it came back with and the first line it printed. It prints where it ran and what the machine calls itself, which is the honest shape of a deploy a demo can perform.".to_string(),
                ),
                (
                    "LIVE LOGS".to_string(),
                    "Opens a real terminal in the deploy directory with `rustx::terminal::open`, and leaves it to you.".to_string(),
                ),
                (
                    "THE DIRECTORY".to_string(),
                    "Both use the deploy directory, which starts as the directory the application was launched from and is changed with the system's own folder picker from CONFIGURE or from the SETTINGS tab.".to_string(),
                ),
            ],
        },
        Topic {
            slug: "routes".to_string(),
            title: "Every screen, by address".to_string(),
            blurb: "The route tree this console is built on.".to_string(),
            body: vec![
                (
                    "THE ORGANISATION".to_string(),
                    "`/` is Home. `/services` lists the organisation's services. `/tasks`, `/docs`, `/status` and `/changelog` are the rest of the top bar, and each one is a page with its own content.".to_string(),
                ),
                (
                    "A SERVICE".to_string(),
                    "`/services/<name>` is OVERVIEW; `/services/<name>/resource`, `/analytics` and `/settings` are the other three tabs. They are four routes and not four states of one screen, so the address says which one you are on and the tab strip reads the address back.".to_string(),
                ),
                (
                    "THE DOCUMENTATION".to_string(),
                    "`/docs` is the index and `/docs/<topic>` is one topic. A slug nobody wrote shows the index again with a line saying so, rather than an error.".to_string(),
                ),
            ],
        },
        Topic {
            slug: "limits".to_string(),
            title: "Limits and what they mean".to_string(),
            blurb: "The three editable fields at the foot of the resource cards.".to_string(),
            body: vec![
                (
                    "WHERE THEY START".to_string(),
                    "The processor limit starts at the number of cores this machine has and the memory limit at its total memory in megabytes, both read the first time the machine is sampled rather than hard-coded.".to_string(),
                ),
                (
                    "WHAT THEY DO".to_string(),
                    "They are the quota the organisation has given the service. This console cannot enforce a quota on a machine it does not own, so a limit is held and shown, not applied — which is why the console says 'limit' and not 'cap'.".to_string(),
                ),
            ],
        },
        Topic {
            slug: "what-is-not-real".to_string(),
            title: "What is not real".to_string(),
            blurb: "The parts that are this application's own copy, named.".to_string(),
            body: vec![
                (
                    "THE SERVICES".to_string(),
                    "The four services, their names, descriptions and health are the application's own list. There is no control plane behind a local demo, and inventing one that looked live would be the one thing this console must not do.".to_string(),
                ),
                (
                    "THE CHANGELOG".to_string(),
                    "The releases are copy. The filter over them, and the button that puts a release on the clipboard, are not.".to_string(),
                ),
                (
                    "EVERYTHING ELSE".to_string(),
                    "Every number with a unit on it — percentages, cores, gigabytes, uptime, the process list — is read from this machine as the screen draws.".to_string(),
                ),
            ],
        },
    ]
}

/// The topic of this slug, if one was written.
pub fn topic_named(slug: &str) -> Option<Topic> {
    topics().into_iter().find(|topic| topic.slug == slug)
}

/// The service of this slug, or the first one.
pub fn service_named(slug: &str) -> Service {
    let all = services();
    all.iter()
        .find(|service| service.slug == slug)
        .cloned()
        .unwrap_or_else(|| all[0].clone())
}

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Ops Monitor")
            .size(1320, 880)
            .start("/services/runner-gateway/resource")
            .description("Resource tracking for the agents running on this machine.")
            .publisher("rustx"),
    )?;

    app.run()
}
