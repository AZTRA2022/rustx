//! Utility Hub — "Reelhouse", the production office of a film unit.
//!
//! Four panes: the rail, the departments, the work, and what just
//! happened. Every screen the rail names is a route of its own, so the
//! address bar says where you are and the back button means something.
//!
//! The production's own lists — departments, crew, call sheets,
//! budget lines — are this application's copy, because there is no
//! production office behind a local demo. Everything *else* leaves the
//! app and touches the machine:
//!
//!   Import      a real file picker, a real CSV read off disk, and the
//!               people in it joining the department that is open
//!   Export      a real save dialog, a real file written, announced
//!               with a real desktop notification
//!   copy        the person's address on the real clipboard
//!   notify      a real desktop notification addressed to that person —
//!               which is what a local demo can honestly send, where
//!               the product would send mail
//!   Preferences a real autostart entry, the system's own theme,
//!               locale and timezone, and a real terminal
//!
//! The types below live here rather than in a page because several
//! pages share values of them: a value a page declares is kept by its
//! name for the whole application, and a type declared inside one page
//! module would be a different type in the next one.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


use rustx::{App, Config};

/// One member of the unit.
#[derive(Clone)]
pub struct Person {
    /// The local part of the address, which is what `/crew/<id>` holds.
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

/// One department of the production.
#[derive(Clone)]
pub struct Department {
    pub slug: String,
    pub name: String,
    pub crew: Vec<Person>,
}

/// A line of Recent Activity.
#[derive(Clone)]
pub struct Happened {
    pub who: String,
    pub what: String,
    pub when_said: String,
    pub week: bool,
}

/// One day on the schedule.
#[derive(Clone)]
pub struct CallSheet {
    pub slug: String,
    pub day: String,
    pub date: String,
    pub unit: String,
    pub location: String,
    pub call: String,
    pub wrap: String,
    pub scenes: Vec<String>,
    pub locked: bool,
}

/// A clip in the footage library.
#[derive(Clone)]
pub struct Clip {
    pub reel: String,
    pub scene: String,
    pub take: u32,
    pub length: String,
    pub department: String,
    pub graded: bool,
}

/// A line of the budget.
#[derive(Clone)]
pub struct BudgetLine {
    pub head: String,
    pub department: String,
    pub planned: f64,
    pub spent: f64,
}

/// A page of the help centre.
#[derive(Clone)]
pub struct HelpTopic {
    pub slug: String,
    pub title: String,
    pub blurb: String,
    pub body: Vec<(String, String)>,
}

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Reelhouse")
            .size(1180, 760)
            .start("/departments/camera")
            .description("The production office of a film unit, in one window.")
            .publisher("rustx"),
    )?;

    app.run()
}

// ---- the production's own lists ---------------------------------------

pub fn starting_departments() -> Vec<Department> {
    vec![
        Department {
            slug: "camera".to_string(),
            name: "Camera".to_string(),
            crew: named(&[
                ("Nadia Okafor", "nadia", "Lead"),
                ("Tom Rivers", "tom", "Operator"),
                ("Aline Sousa", "aline", "Operator"),
                ("Marek Kowal", "marek", "Trainee"),
                ("Priya Menon", "priya", "Lead"),
                ("Jonah Blake", "jonah", "Operator"),
                ("Esme Laurent", "esme", "Operator"),
                ("Kofi Mensah", "kofi", "Trainee"),
                ("Rosa Delgado", "rosa", "Operator"),
                ("Ilya Petrov", "ilya", "Operator"),
                ("Freya Lund", "freya", "Trainee"),
                ("Sam Whitaker", "sam", "Operator"),
            ]),
        },
        Department {
            slug: "lighting".to_string(),
            name: "Lighting".to_string(),
            crew: built(
                "light",
                &[
                    "Gaffer", "Best Boy", "Spark", "Rigger", "Board Op", "Spark", "Spark",
                    "Rigger", "Spark", "Trainee",
                ],
            ),
        },
        Department {
            slug: "sound".to_string(),
            name: "Sound".to_string(),
            crew: built(
                "sound",
                &["Mixer", "Boom", "Utility", "Mixer", "Boom", "Utility"],
            ),
        },
        Department {
            slug: "art-department".to_string(),
            name: "Art Department".to_string(),
            crew: built(
                "art",
                &[
                    "Designer",
                    "Buyer",
                    "Dresser",
                    "Standby",
                    "Buyer",
                    "Dresser",
                    "Standby",
                    "Carpenter",
                    "Painter",
                    "Trainee",
                    "Runner",
                    "Runner",
                ],
            ),
        },
        Department {
            slug: "post-production".to_string(),
            name: "Post Production".to_string(),
            crew: built(
                "post",
                &[
                    "Editor",
                    "Assistant",
                    "Colourist",
                    "Sound Ed",
                    "VFX",
                    "VFX",
                    "Online",
                    "Runner",
                ],
            ),
        },
        Department {
            slug: "locations".to_string(),
            name: "Locations".to_string(),
            crew: built("loc", &["Manager", "Assistant", "Scout", "Unit", "Marshal"]),
        },
    ]
}

/// The camera unit, named one by one because it is the department the
/// application opens on and the one the screenshots show.
fn named(people: &[(&str, &str, &str)]) -> Vec<Person> {
    people
        .iter()
        .map(|(name, handle, role)| Person {
            id: handle.to_string(),
            name: name.to_string(),
            email: format!("{handle}@reelhouse.co"),
            role: role.to_string(),
        })
        .collect()
}

/// A department's crew, named from its own initial so that two
/// departments are never the same list.
fn built(prefix: &str, jobs: &[&str]) -> Vec<Person> {
    let firsts = [
        "Ada", "Ben", "Cleo", "Dev", "Enzo", "Fay", "Gus", "Hana", "Ivo", "Juno", "Kit", "Lio",
    ];
    let lasts = [
        "Arden", "Bly", "Cross", "Dane", "Ebe", "Falk", "Gray", "Holt", "Ives", "Jarl", "Kerr",
        "Lund",
    ];
    jobs.iter()
        .enumerate()
        .map(|(index, job)| Person {
            id: format!("{prefix}{}", index + 1),
            name: format!(
                "{} {}",
                firsts[index % firsts.len()],
                lasts[(index + prefix.len()) % lasts.len()]
            ),
            email: format!("{}{}@reelhouse.co", prefix, index + 1),
            role: if index == 0 {
                "Lead".to_string()
            } else if *job == "Trainee" || *job == "Runner" {
                "Trainee".to_string()
            } else {
                "Operator".to_string()
            },
        })
        .collect()
}

pub fn starting_activity() -> Vec<Happened> {
    vec![
        Happened {
            who: "Nadia Okafor".to_string(),
            what: "added a B-cam operator".to_string(),
            when_said: "2 hrs".to_string(),
            week: false,
        },
        Happened {
            who: "Tom Rivers".to_string(),
            what: "logged 4 magazines".to_string(),
            when_said: "3 hrs".to_string(),
            week: false,
        },
        Happened {
            who: "Aline Sousa".to_string(),
            what: "updated the lens list".to_string(),
            when_said: "5 hrs".to_string(),
            week: false,
        },
        Happened {
            who: "Marek Kowal".to_string(),
            what: "completed safety induction".to_string(),
            when_said: "1 day".to_string(),
            week: true,
        },
        Happened {
            who: "Priya Menon".to_string(),
            what: "moved call time to 06:30".to_string(),
            when_said: "2 days".to_string(),
            week: true,
        },
        Happened {
            who: "Jonah Blake".to_string(),
            what: "flagged a gate check".to_string(),
            when_said: "3 days".to_string(),
            week: true,
        },
        Happened {
            who: "Esme Laurent".to_string(),
            what: "uploaded dailies".to_string(),
            when_said: "4 days".to_string(),
            week: true,
        },
        Happened {
            who: "Kofi Mensah".to_string(),
            what: "requested a kit swap".to_string(),
            when_said: "5 days".to_string(),
            week: true,
        },
    ]
}

pub fn call_sheets() -> Vec<CallSheet> {
    vec![
        CallSheet {
            slug: "day-18".to_string(),
            day: "Day 18".to_string(),
            date: "Monday".to_string(),
            unit: "Main unit".to_string(),
            location: "Quayside warehouse, stage 2".to_string(),
            call: "06:30".to_string(),
            wrap: "19:00".to_string(),
            scenes: vec![
                "42 · int. warehouse · day".to_string(),
                "42A · insert, hands".to_string(),
                "43 · int. corridor · day".to_string(),
            ],
            locked: true,
        },
        CallSheet {
            slug: "day-19".to_string(),
            day: "Day 19".to_string(),
            date: "Tuesday".to_string(),
            unit: "Main unit".to_string(),
            location: "Quayside warehouse, stage 2".to_string(),
            call: "07:00".to_string(),
            wrap: "19:30".to_string(),
            scenes: vec![
                "44 · int. office · night".to_string(),
                "45 · int. office · night".to_string(),
            ],
            locked: true,
        },
        CallSheet {
            slug: "day-20".to_string(),
            day: "Day 20".to_string(),
            date: "Wednesday".to_string(),
            unit: "Second unit".to_string(),
            location: "Harbour road, east end".to_string(),
            call: "05:45".to_string(),
            wrap: "16:00".to_string(),
            scenes: vec![
                "12B · ext. road · dawn".to_string(),
                "12C · ext. road · dawn".to_string(),
                "13 · ext. junction".to_string(),
            ],
            locked: false,
        },
        CallSheet {
            slug: "day-21".to_string(),
            day: "Day 21".to_string(),
            date: "Thursday".to_string(),
            unit: "Main unit".to_string(),
            location: "Riverside house, upper floor".to_string(),
            call: "08:00".to_string(),
            wrap: "20:00".to_string(),
            scenes: vec!["51 · int. bedroom · night".to_string()],
            locked: false,
        },
        CallSheet {
            slug: "day-22".to_string(),
            day: "Day 22".to_string(),
            date: "Friday".to_string(),
            unit: "Main unit".to_string(),
            location: "Riverside house, garden".to_string(),
            call: "07:30".to_string(),
            wrap: "18:30".to_string(),
            scenes: vec![
                "52 · ext. garden · day".to_string(),
                "53 · ext. garden · dusk".to_string(),
            ],
            locked: false,
        },
    ]
}

pub fn call_sheet_named(slug: &str) -> Option<CallSheet> {
    call_sheets().into_iter().find(|sheet| sheet.slug == slug)
}

pub fn footage() -> Vec<Clip> {
    let scenes = [
        "42", "42A", "43", "44", "45", "12B", "12C", "13", "51", "52", "53", "54",
    ];
    let departments = [
        "Camera",
        "Camera",
        "Second unit",
        "Camera",
        "Sound",
        "Camera",
    ];
    scenes
        .iter()
        .enumerate()
        .flat_map(|(index, scene)| {
            (1..=3u32).map(move |take| Clip {
                reel: format!("A{:03}", index * 3 + take as usize),
                scene: scene.to_string(),
                take,
                length: format!(
                    "{:01}:{:02}",
                    1 + (index + take as usize) % 4,
                    (index * 13 + take as usize * 7) % 60
                ),
                department: departments[index % departments.len()].to_string(),
                graded: (index + take as usize).is_multiple_of(3),
            })
        })
        .collect()
}

pub fn budget_lines() -> Vec<BudgetLine> {
    vec![
        BudgetLine {
            head: "Camera package".to_string(),
            department: "Camera".to_string(),
            planned: 184_000.0,
            spent: 171_400.0,
        },
        BudgetLine {
            head: "Lenses and filters".to_string(),
            department: "Camera".to_string(),
            planned: 62_000.0,
            spent: 64_900.0,
        },
        BudgetLine {
            head: "Lighting hire".to_string(),
            department: "Lighting".to_string(),
            planned: 148_000.0,
            spent: 121_300.0,
        },
        BudgetLine {
            head: "Generators and power".to_string(),
            department: "Lighting".to_string(),
            planned: 41_000.0,
            spent: 38_200.0,
        },
        BudgetLine {
            head: "Sound package".to_string(),
            department: "Sound".to_string(),
            planned: 54_000.0,
            spent: 49_750.0,
        },
        BudgetLine {
            head: "Set build".to_string(),
            department: "Art Department".to_string(),
            planned: 236_000.0,
            spent: 248_600.0,
        },
        BudgetLine {
            head: "Dressing and props".to_string(),
            department: "Art Department".to_string(),
            planned: 88_000.0,
            spent: 72_100.0,
        },
        BudgetLine {
            head: "Edit suite".to_string(),
            department: "Post Production".to_string(),
            planned: 96_000.0,
            spent: 40_000.0,
        },
        BudgetLine {
            head: "Grade and online".to_string(),
            department: "Post Production".to_string(),
            planned: 74_000.0,
            spent: 0.0,
        },
        BudgetLine {
            head: "Location fees".to_string(),
            department: "Locations".to_string(),
            planned: 112_000.0,
            spent: 103_450.0,
        },
        BudgetLine {
            head: "Unit and parking".to_string(),
            department: "Locations".to_string(),
            planned: 37_000.0,
            spent: 35_900.0,
        },
    ]
}

pub fn help_topics() -> Vec<HelpTopic> {
    vec![
        HelpTopic {
            slug: "departments".to_string(),
            title: "Departments and crew".to_string(),
            blurb: "How the unit is laid out, and what the three tabs hold.".to_string(),
            body: vec![
                ("THE LIST".to_string(), "Every department is a route: /departments/<slug>. The pane in the middle filters that list as you type, and the department you are on is the one in the address bar, so a reload or a back button lands where you left.".to_string()),
                ("THE TABS".to_string(), "Crew, Call Times and Kit are three routes of the same department rather than three states of one screen. Crew is the roster; Call Times is the same roster against the unit's call; Kit is what each person has signed out.".to_string()),
                ("ONE PERSON".to_string(), "A row of the roster opens /crew/<id>, which is that person's own page: their department, their role, their call, and the three actions the roster offers.".to_string()),
            ],
        },
        HelpTopic {
            slug: "import-export".to_string(),
            title: "Importing and exporting".to_string(),
            blurb: "What really happens to a file you point the app at.".to_string(),
            body: vec![
                ("IMPORT".to_string(), "Import opens the system's own file picker, reads the chosen CSV off disk and adds its rows to the department that is open. A row is name,email,role; the first line is treated as a header and skipped. Nothing is uploaded anywhere: the file is read and the rows join the list in memory.".to_string()),
                ("EXPORT".to_string(), "Export opens the system's own save dialog and writes a real CSV where you point it, then raises a desktop notification and asks the shell to remember the file in the recent list.".to_string()),
            ],
        },
        HelpTopic {
            slug: "notifications".to_string(),
            title: "Call sheets and notifications".to_string(),
            blurb: "What the mail button does, and what it cannot do.".to_string(),
            body: vec![
                ("WHAT IT SENDS".to_string(), "The mail button raises a real desktop notification from this machine saying the call sheet went to that person. It does not send mail: a local demo has no mail server behind it, and pretending otherwise would be the one thing this app must not do.".to_string()),
                ("WHAT IT RECORDS".to_string(), "Every action also files a line in Recent Activity, which is the app's own state and says so.".to_string()),
            ],
        },
        HelpTopic {
            slug: "the-machine".to_string(),
            title: "Preferences and this machine".to_string(),
            blurb: "The only screen that is about the computer rather than the production.".to_string(),
            body: vec![
                ("AUTOSTART".to_string(), "The switch writes a real entry under the system's own startup list through rustx::autostart, and reads it back to decide whether it is on. On a platform that has no such list, it says so instead of showing a switch that does nothing.".to_string()),
                ("WHAT IS READ".to_string(), "The system theme, the locale, the timezone and the shell are read from the machine as the screen draws. The terminal button opens a real terminal in the directory the application was launched from.".to_string()),
            ],
        },
    ]
}

pub fn help_topic_named(slug: &str) -> Option<HelpTopic> {
    help_topics().into_iter().find(|topic| topic.slug == slug)
}

// ---- small pure helpers the pages share -------------------------------

/// A department name as it appears in an address.
pub fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|letter| {
            if letter.is_ascii_alphanumeric() {
                letter
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

pub fn role_background(role: &str) -> &'static str {
    match role {
        "Lead" => "#11291d",
        "Trainee" => "#241c38",
        _ => "#1a1f27",
    }
}

pub fn role_ink(role: &str) -> &'static str {
    match role {
        "Lead" => "#4ec98a",
        "Trainee" => "#a78bfa",
        _ => "#9aa5b1",
    }
}

/// The paint of a round avatar, as a style declaration. The colour is
/// the name's own, so a list of people has faces without a list of
/// files.
pub fn avatar_paint(name: &str, size: u32) -> String {
    let seed: u32 = name.bytes().map(|byte| byte as u32).sum();
    let hue = seed % 360;
    format!(
        "display:inline-flex;align-items:center;justify-content:center;flex:none;width:{size}px;height:{size}px;border-radius:50%;background:linear-gradient(140deg, hsl({hue} 42% 38%), hsl({} 42% 24%));color:#eaf0f6;font-size:{}px;font-weight:600",
        (hue + 40) % 360,
        (size as f64 * 0.42).round() as u32
    )
}

/// What the machine says about itself, for Preferences.
pub fn read_machine() -> Vec<(String, String)> {
    let theme = match rustx::os::system::theme() {
        Ok(theme) => format!("{theme:?}").to_lowercase(),
        Err(_) => "unknown".to_string(),
    };
    let shell = match rustx::terminal::shell() {
        Ok(shell) => shell.name,
        Err(_) => "none found".to_string(),
    };
    vec![
        ("system theme".to_string(), theme),
        (
            "locale".to_string(),
            rustx::os::system::locale().unwrap_or_else(|_| "unknown".to_string()),
        ),
        (
            "timezone".to_string(),
            rustx::os::system::timezone().unwrap_or_else(|_| "unknown".to_string()),
        ),
        ("shell".to_string(), shell),
        (
            "working directory".to_string(),
            std::env::current_dir()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
        ),
    ]
}
