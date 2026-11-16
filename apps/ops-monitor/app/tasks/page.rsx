// TASKS — the jobs this console can run, and what is running on the
// machine right now.
//
// A job here is a real command. RUN hands it to the platform's own
// shell through `rustx::terminal::run`, waits for it, and shows the
// exit code and the first lines it printed. Nothing is simulated: a
// job that fails says so with the shell's own code.

/// One job of the console, and the last thing it said.
#[derive(Clone)]
struct Job {
    name: String,
    about: String,
    script: String,
    said: String,
    code: i32,
    ran: bool,
}

let mut processes: Vec<(String, f64, u64)> = Vec::new();
let mut cpu: f64 = 0.0;
let mut samples: u64 = 0;

let mut jobs: Vec<Job> = Vec::new();
let mut busy: bool = false;

fn Page() -> Element {
    effect((), || {
        if jobs.is_empty() {
            jobs = starting_jobs();
        }
    });

    return <div key="tasks" id="screen-tasks" className="pt-5">
        <div className="flex items-end">
            <div>
                <h1 className="text-[15px] font-semibold tracking-tight">Tasks</h1>
                <p className="mt-1 text-[11px] text-muted">
                    Every job below is a real command handed to this machine's shell.
                </p>
            </div>
            <button
                className="ml-auto flex items-center gap-1.5 rounded-[5px] border border-line bg-[#262626] px-3 py-[7px] text-[10px] tracking-[0.1em] text-foreground hover:border-faint disabled:opacity-50"
                disabled={busy}
                onClick={move |_| run_all()}
            >
                <Play size={11} color="#d6d6d4" />
                {if busy { "RUNNING…" } else { "RUN ALL" }}
            </button>
        </div>

        <div id="jobs" className="mt-4 flex flex-col gap-2">
            {jobs.iter().enumerate().map(|(index, job)| JobRow(index, job.clone())).collect::<Vec<_>>()}
        </div>

        <h2 className="mt-7 text-[10px] tracking-[0.14em] text-muted">RUNNING ON THIS MACHINE</h2>
        <div className="mt-3 overflow-hidden rounded-[7px] border border-line bg-panel">
            {processes.iter().map(|row| ProcessRow(row.0.clone(), row.1, row.2)).collect::<Vec<_>>()}
            {if processes.is_empty() {
                <div className="px-4 py-6 text-[11px] text-faint">The first reading has not landed yet.</div>
            } else {
                <span></span>
            }}
        </div>
        <p className="mt-3 text-[10px] text-faint">
            {format!("{} reading(s) taken · the processor is at {:.0}% as this drew.", samples, cpu)}
        </p>
    </div>;
}

fn JobRow(index: usize, job: Job) -> Element {
    let ink = if !job.ran {
        "text-faint"
    } else if job.code == 0 {
        "text-[#5bbd7f]"
    } else {
        "text-[#e0533c]"
    };
    let state = format!("shrink-0 text-[10px] tracking-[0.1em] {ink}");

    return <div key={job.name.clone()} className="rounded-[7px] border border-line bg-panel p-4">
        <div className="flex items-center gap-3">
            <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-[5px] bg-[#232323]">
                <Terminal size={12} color="#8a8a85" />
            </span>
            <span className="min-w-0">
                <span className="block text-[12px]">{job.name.clone()}</span>
                <span className="mt-0.5 block text-[10px] text-muted">{job.about.clone()}</span>
            </span>
            <span className={state}>
                {if !job.ran {
                    "NEVER RUN".to_string()
                } else {
                    format!("EXIT {}", job.code)
                }}
            </span>
            <button
                className="shrink-0 rounded-[5px] border border-line bg-panel2 px-3 py-[6px] text-[10px] tracking-[0.1em] text-muted hover:text-foreground disabled:opacity-50"
                disabled={busy}
                onClick={move |_| run_one(index)}
            >
                RUN
            </button>
        </div>
        <pre className="mt-3 max-h-24 overflow-auto whitespace-pre-wrap rounded-[5px] border border-linesoft bg-[#121212] px-3 py-2 text-[10px] leading-[1.5] text-muted">
            {if job.said.is_empty() { job.script.clone() } else { job.said.clone() }}
        </pre>
    </div>;
}

fn ProcessRow(name: String, load: f64, memory: u64) -> Element {
    return <div key={name.clone()} className="grid grid-cols-[1fr_80px_90px] items-center border-b border-linesoft px-4 py-2 text-[11px] last:border-b-0">
        <span className="truncate pr-3">{name}</span>
        <span className="text-right tabular-nums text-[#9fb8dc]">{format!("{:.1}%", load)}</span>
        <span className="text-right tabular-nums text-muted">{format!("{:.2} GB", memory as f64 / 1_073_741_824.0)}</span>
    </div>;
}

// ---- what the buttons do ---------------------------------------------

fn run_one(index: usize) {
    busy = true;
    let mut all = jobs.clone();
    if let Some(job) = all.get_mut(index) {
        match rustx::terminal::run(&job.script, None) {
            Ok(output) => {
                let said = if output.out.trim().is_empty() { output.err.clone() } else { output.out.clone() };
                job.said = said.lines().take(6).collect::<Vec<_>>().join("\n");
                job.code = output.code;
            }
            Err(error) => {
                job.said = format!("the shell refused: {error}");
                job.code = -1;
            }
        }
        job.ran = true;
    }
    jobs = all;
    busy = false;
}

fn run_all() {
    let count = jobs.len();
    for index in 0..count {
        run_one(index);
    }
    let failed = jobs.iter().filter(|job| job.ran && job.code != 0).count();
    let _ = rustx::notify::show(rustx::notify::What::new(
        "FORGE ORG · tasks",
        format!("{count} job(s) ran, {failed} came back non-zero."),
    ));
}

// ---- plain functions --------------------------------------------------

/// Three jobs that mean something on either platform, written in the
/// shell `rustx::terminal` actually found.
#[cfg(windows)]
fn starting_jobs() -> Vec<Job> {
    vec![
        Job {
            name: "Where are we".to_string(),
            about: "Prints the working directory and the machine's name.".to_string(),
            script: "\"$(Get-Location) on $env:COMPUTERNAME\"".to_string(),
            said: String::new(),
            code: 0,
            ran: false,
        },
        Job {
            name: "Disk headroom".to_string(),
            about: "Free space on every fixed drive, in gigabytes.".to_string(),
            script: "Get-PSDrive -PSProvider FileSystem | Where-Object { $_.Used -ne $null } | ForEach-Object { \"$($_.Name): $([math]::Round($_.Free/1GB,1)) GB free\" }".to_string(),
            said: String::new(),
            code: 0,
            ran: false,
        },
        Job {
            name: "Uptime".to_string(),
            about: "How long this machine has been up.".to_string(),
            script: "$b = (Get-CimInstance Win32_OperatingSystem).LastBootUpTime; \"up since $b\"".to_string(),
            said: String::new(),
            code: 0,
            ran: false,
        },
    ]
}

#[cfg(not(windows))]
fn starting_jobs() -> Vec<Job> {
    vec![
        Job {
            name: "Where are we".to_string(),
            about: "Prints the working directory and the machine's name.".to_string(),
            script: "echo \"$(pwd) on $(hostname)\"".to_string(),
            said: String::new(),
            code: 0,
            ran: false,
        },
        Job {
            name: "Disk headroom".to_string(),
            about: "Free space on every mounted filesystem.".to_string(),
            script: "df -h | head -8".to_string(),
            said: String::new(),
            code: 0,
            ran: false,
        },
        Job {
            name: "Uptime".to_string(),
            about: "How long this machine has been up.".to_string(),
            script: "uptime".to_string(),
            said: String::new(),
            code: 0,
            ran: false,
        },
    ]
}
