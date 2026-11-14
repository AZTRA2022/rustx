// REPORTS — the production counted, and drawn.
//
// Both charts are `recharts`, off npm, fed from the live department
// list and the schedule rather than from a fixture: importing a crew
// list moves the bars.

let mut departments: Vec<Department> = Vec::new();
let mut note: String = String::new();

/// A bar of the department chart. `ToValue` is what lets a Rust value
/// cross into a JavaScript chart library.
#[derive(rustx::ToValue, Clone)]
struct Unit {
    name: String,
    people: f64,
    leads: f64,
    trainees: f64,
}

/// A point of the clip chart.
#[derive(rustx::ToValue, Clone)]
struct Scene {
    scene: String,
    clips: f64,
    graded: f64,
}

fn Page() -> Element {
    let units: Vec<Unit> = departments
        .iter()
        .map(|department| Unit {
            name: department.name.clone(),
            people: department.crew.len() as f64,
            leads: department.crew.iter().filter(|person| person.role == "Lead").count() as f64,
            trainees: department.crew.iter().filter(|person| person.role == "Trainee").count() as f64,
        })
        .collect();

    let clips = footage();
    let mut scenes: Vec<Scene> = Vec::new();
    for clip in clips.iter() {
        match scenes.iter_mut().find(|held| held.scene == clip.scene) {
            Some(held) => {
                held.clips += 1.0;
                if clip.graded {
                    held.graded += 1.0;
                }
            }
            None => scenes.push(Scene {
                scene: clip.scene.clone(),
                clips: 1.0,
                graded: if clip.graded { 1.0 } else { 0.0 },
            }),
        }
    }

    let people: usize = departments.iter().map(|department| department.crew.len()).sum();

    return <div key="reports" className="px-5 py-4">
        <div className="flex items-start">
            <div>
                <h1 className="text-[13px] font-semibold text-foreground">Reports</h1>
                <p className="mt-0.5 text-[11px] text-muted">
                    {format!("{people} people across {} departments, {} clips across {} scenes.", departments.len(), clips.len(), scenes.len())}
                </p>
            </div>
            <button
                className="ml-auto rounded-md border border-border bg-[#11151b] px-2.5 py-[6px] text-[11px] text-foreground hover:border-faint"
                onClick={move |_| export_report()}
            >
                Export the figures
            </button>
        </div>

        <div className="mt-4 rounded-md border border-border bg-[#0f131a] p-4">
            <div className="text-[9px] tracking-[0.14em] text-faint">HEADCOUNT BY DEPARTMENT</div>
            <div className="mt-3">
                <recharts::BarChart width={620} height={210} data={units}>
                    <recharts::CartesianGrid stroke="#1a1f27" vertical={false} />
                    <recharts::XAxis dataKey="name" stroke="#1a1f27" tickLine={false} />
                    <recharts::YAxis stroke="#1a1f27" tickLine={false} width={30} />
                    <recharts::Tooltip isAnimationActive={false} cursor={false} />
                    <recharts::Bar dataKey="people" name="People" fill="#2f6fed" isAnimationActive={false} />
                    <recharts::Bar dataKey="leads" name="Leads" fill="#4ec98a" isAnimationActive={false} />
                    <recharts::Bar dataKey="trainees" name="Trainees" fill="#a78bfa" isAnimationActive={false} />
                </recharts::BarChart>
            </div>
        </div>

        <div className="mt-3 rounded-md border border-border bg-[#0f131a] p-4">
            <div className="text-[9px] tracking-[0.14em] text-faint">CLIPS AND GRADES BY SCENE</div>
            <div className="mt-3">
                <recharts::LineChart width={620} height={190} data={scenes}>
                    <recharts::CartesianGrid stroke="#1a1f27" vertical={false} />
                    <recharts::XAxis dataKey="scene" stroke="#1a1f27" tickLine={false} />
                    <recharts::YAxis stroke="#1a1f27" tickLine={false} width={30} />
                    <recharts::Tooltip isAnimationActive={false} cursor={false} />
                    <recharts::Line type="monotone" dataKey="clips" name="Clips" stroke="#2f6fed" strokeWidth={1.5} dot={false} isAnimationActive={false} />
                    <recharts::Line type="monotone" dataKey="graded" name="Graded" stroke="#4ec98a" strokeWidth={1.5} dot={false} isAnimationActive={false} />
                </recharts::LineChart>
            </div>
        </div>
    </div>;
}

/// A real CSV of the figures on this page, where the system's own
/// dialog puts it.
fn export_report() {
    let ask = rustx::dialog::Ask::new("Export the figures")
        .filter("Comma separated", ["csv"])
        .named("reelhouse-report.csv");
    let Ok(path) = rustx::dialog::save(ask) else {
        note = "the export was cancelled".to_string();
        return;
    };
    let mut text = String::from("department,people,leads,trainees\n");
    for department in departments.iter() {
        text.push_str(&format!(
            "{},{},{},{}\n",
            department.name,
            department.crew.len(),
            department.crew.iter().filter(|person| person.role == "Lead").count(),
            department.crew.iter().filter(|person| person.role == "Trainee").count()
        ));
    }
    match std::fs::write(&path, text) {
        Ok(()) => {
            note = format!("the figures are at {}", path.display());
            let _ = rustx::os::shell::remember_recent(&path);
        }
        Err(error) => note = format!("the export failed: {error}"),
    }
}
