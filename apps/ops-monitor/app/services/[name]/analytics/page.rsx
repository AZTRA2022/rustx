// ANALYTICS — the same readings as RESOURCE, read over time instead
// of as a gauge.
//
// The three figures at the top are computed over the history the root
// layout keeps, so they change as the window stays open; the bar chart
// is the heaviest processes on this machine at the instant it draws.

let mut history: Vec<Sample> = Vec::new();
let mut processes: Vec<(String, f64, u64)> = Vec::new();
let mut samples: u64 = 0;
let mut cores: usize = 0;
let mut mem_total: u64 = 0;

/// Which of the three series the big chart fills. A route's tab would
/// be overkill for a three-way switch inside one screen.
let mut series: String = "cpu_pct".to_string();

fn Page(name: String) -> Element {
    let _ = name;
    let all = history.clone();
    let load: Vec<f64> = all.iter().map(|sample| sample.cpu_pct).collect();

    return <div key="analytics" id="screen-analytics">
        <div className="mt-5 flex items-center">
            <h2 className="text-[10px] tracking-[0.14em] text-muted">ANALYTICS</h2>
            <div className="ml-auto flex rounded-[5px] border border-line bg-panel p-[3px]">
                {SeriesTab("CPU".to_string(), "cpu_pct".to_string())}
                {SeriesTab("GPU".to_string(), "gpu_pct".to_string())}
                {SeriesTab("MEMORY".to_string(), "mem_pct".to_string())}
            </div>
        </div>

        <div className="mt-3 grid grid-cols-4 gap-3">
            {Figure("SAMPLES".to_string(), samples.to_string(), "one per second since this window opened".to_string())}
            {Figure("MEAN".to_string(), percent(mean(&load)), "processor, over the whole history".to_string())}
            {Figure("PEAK".to_string(), percent(peak(&load)), "the highest second so far".to_string())}
            {Figure("P95".to_string(), percent(p95(&load)), "the line 95% of seconds stay under".to_string())}
        </div>

        <div className="mt-4 grid grid-cols-[1.35fr_1fr] gap-4">
            <div className="rounded-[7px] border border-line bg-panel p-4">
                <h3 className="text-[10px] tracking-[0.14em] text-muted">{format!("{} OVER TIME", label(&series))}</h3>
                <div className="mt-3">
                    <recharts::AreaChart width={560} height={210} data={all.clone()}>
                        <recharts::CartesianGrid stroke="#242424" vertical={false} />
                        <recharts::XAxis dataKey="at" stroke="#2b2b2b" tickLine={false} minTickGap={56} />
                        <recharts::YAxis stroke="#2b2b2b" tickLine={false} domain={vec![0.0, 100.0]} width={34} unit="%" />
                        <recharts::Tooltip isAnimationActive={false} cursor={false} />
                        <recharts::Area
                            type="monotone"
                            dataKey={series.clone()}
                            name={label(&series)}
                            stroke={tint(&series)}
                            fill={tint(&series)}
                            fillOpacity={0.14}
                            strokeWidth={1.5}
                            isAnimationActive={false}
                        />
                    </recharts::AreaChart>
                </div>
                <p className="mt-2 text-[10px] text-faint">
                    {format!("{} second(s) of history held, 480 at most.", all.len())}
                </p>
            </div>

            <div className="rounded-[7px] border border-line bg-panel p-4">
                <h3 className="text-[10px] tracking-[0.14em] text-muted">WHERE THE PROCESSOR GOES</h3>
                <div className="mt-3 flex flex-col gap-2">
                    {processes.iter().map(|row| Bar(row.0.clone(), row.1)).collect::<Vec<_>>()}
                    {if processes.is_empty() {
                        <div className="text-[11px] text-faint">The first reading has not landed yet.</div>
                    } else {
                        <span></span>
                    }}
                </div>
                <p className="mt-4 text-[10px] text-faint">
                    {format!("{} cores · {:.0} GB of memory on this machine.", cores, mem_total as f64 / 1_073_741_824.0)}
                </p>
            </div>
        </div>
    </div>;
}

fn SeriesTab(label: String, key: String) -> Element {
    let active = series == key;
    let classes = format!(
        "rounded-[3px] px-3 py-[5px] text-[10px] tracking-[0.1em] {}",
        if active { "bg-[#2e2e2e] text-foreground" } else { "text-muted hover:text-foreground" }
    );
    let chosen = key.clone();
    return <button key={label.clone()} className={classes} onClick={move |_| series = chosen.clone()}>
        {label}
    </button>;
}

fn Figure(name: String, value: String, under: String) -> Element {
    return <div key={name.clone()} className="rounded-[7px] border border-line bg-panel px-4 py-3">
        <div className="text-[9px] tracking-[0.12em] text-faint">{name}</div>
        <div className="mt-1.5 text-[18px] font-semibold tabular-nums">{value}</div>
        <div className="mt-1 text-[10px] leading-snug text-muted">{under}</div>
    </div>;
}

fn Bar(name: String, load: f64) -> Element {
    let width = format!("height:6px;border-radius:3px;background-color:#4a86d6;width:{}%", load.clamp(1.0, 100.0));
    return <div key={name.clone()}>
        <div className="flex items-baseline text-[11px]">
            <span className="truncate pr-2">{name}</span>
            <span className="ml-auto shrink-0 tabular-nums text-muted">{format!("{:.1}%", load)}</span>
        </div>
        <div className="mt-1 h-[6px] rounded-[3px] bg-[#242424]">
            <div style={width}></div>
        </div>
    </div>;
}

// ---- plain functions --------------------------------------------------

fn label(key: &str) -> String {
    match key {
        "gpu_pct" => "GPU".to_string(),
        "mem_pct" => "MEMORY".to_string(),
        _ => "CPU".to_string(),
    }
}

fn tint(key: &str) -> String {
    match key {
        "gpu_pct" => "#4ea972".to_string(),
        "mem_pct" => "#d9793b".to_string(),
        _ => "#4a86d6".to_string(),
    }
}

fn percent(value: f64) -> String {
    format!("{:.1}%", value)
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn peak(values: &[f64]) -> f64 {
    values.iter().copied().fold(0.0_f64, f64::max)
}

fn p95(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((sorted.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
    sorted[index.min(sorted.len() - 1)]
}
