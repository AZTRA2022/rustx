//! R Intelligent Data — the intelligence layer for your data.
//!
//! The front of the application is the product: the architecture, the
//! platform, who it is for, what a developer gets, the documentation,
//! the company, and a way to get in touch. Every one of those is a
//! real route with real content.
//!
//! The call to action is not a dead end either: GET STARTED opens the
//! console at `/console`, which is the part of this application that
//! does work. Tables live in memory as plain Rust; the only two ways
//! data gets in are real ones — dropping a `.csv` on the window, with
//! its real path through `rustx::dropped`, or the system's own file
//! dialog through `rustx::dialog` — and a table can be queried,
//! sorted, charted and written back out to a real file.
//!
//! The headline on the landing page is set in the 5×7 dot-matrix face
//! at the foot of this file, because the typeface the design calls
//! for is a dot-matrix face and there is none on this machine. One
//! round dot per lit cell, laid out by the page.

use rustx::{App, Config};

/// A table the console holds.
///
/// Not `Table`: a library component of that name is in scope inside
/// every page, and a type and a component with the same name is a
/// collision waiting at the worst moment.
#[derive(Clone)]
pub struct Dataset {
    pub slug: String,
    pub name: String,
    pub about: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// A page of the documentation.
#[derive(Clone)]
pub struct Doc {
    pub slug: String,
    pub title: String,
    pub blurb: String,
    pub body: Vec<(String, String)>,
}

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("R Intelligent Data")
            .size(1180, 800)
            .description("The intelligence layer for your data, and the console under it.")
            .publisher("rustx"),
    )?;

    app.run()
}

// ---- what the console starts with -------------------------------------

pub fn starting_tables() -> Vec<Dataset> {
    vec![
        Dataset {
            slug: "users".to_string(),
            name: "users".to_string(),
            about: "Every account the platform has issued, with the plan it sits on.".to_string(),
            columns: vec![
                "id".to_string(),
                "name".to_string(),
                "plan".to_string(),
                "region".to_string(),
            ],
            rows: vec![
                vec![
                    "1".to_string(),
                    "Ada".to_string(),
                    "pro".to_string(),
                    "eu-west".to_string(),
                ],
                vec![
                    "2".to_string(),
                    "Grace".to_string(),
                    "free".to_string(),
                    "us-east".to_string(),
                ],
                vec![
                    "3".to_string(),
                    "Linus".to_string(),
                    "pro".to_string(),
                    "eu-north".to_string(),
                ],
                vec![
                    "4".to_string(),
                    "Radia".to_string(),
                    "team".to_string(),
                    "us-west".to_string(),
                ],
                vec![
                    "5".to_string(),
                    "Barbara".to_string(),
                    "pro".to_string(),
                    "eu-west".to_string(),
                ],
                vec![
                    "6".to_string(),
                    "Alan".to_string(),
                    "free".to_string(),
                    "ap-south".to_string(),
                ],
                vec![
                    "7".to_string(),
                    "Karen".to_string(),
                    "team".to_string(),
                    "us-east".to_string(),
                ],
                vec![
                    "8".to_string(),
                    "Edsger".to_string(),
                    "pro".to_string(),
                    "eu-north".to_string(),
                ],
            ],
        },
        Dataset {
            slug: "orders".to_string(),
            name: "orders".to_string(),
            about: "Every charge raised against an account, in the account's own currency."
                .to_string(),
            columns: vec![
                "id".to_string(),
                "user_id".to_string(),
                "total".to_string(),
                "state".to_string(),
            ],
            rows: vec![
                vec![
                    "101".to_string(),
                    "1".to_string(),
                    "42.00".to_string(),
                    "settled".to_string(),
                ],
                vec![
                    "102".to_string(),
                    "3".to_string(),
                    "19.90".to_string(),
                    "settled".to_string(),
                ],
                vec![
                    "103".to_string(),
                    "4".to_string(),
                    "148.00".to_string(),
                    "pending".to_string(),
                ],
                vec![
                    "104".to_string(),
                    "1".to_string(),
                    "42.00".to_string(),
                    "settled".to_string(),
                ],
                vec![
                    "105".to_string(),
                    "7".to_string(),
                    "96.50".to_string(),
                    "failed".to_string(),
                ],
                vec![
                    "106".to_string(),
                    "8".to_string(),
                    "19.90".to_string(),
                    "settled".to_string(),
                ],
            ],
        },
        Dataset {
            slug: "signals".to_string(),
            name: "signals".to_string(),
            about: "What the layer saw and kept: one row per event that cleared the noise floor."
                .to_string(),
            columns: vec![
                "at".to_string(),
                "source".to_string(),
                "kind".to_string(),
                "score".to_string(),
            ],
            rows: vec![
                vec![
                    "06:12".to_string(),
                    "edge-1".to_string(),
                    "latency".to_string(),
                    "0.82".to_string(),
                ],
                vec![
                    "06:41".to_string(),
                    "edge-3".to_string(),
                    "error-rate".to_string(),
                    "0.91".to_string(),
                ],
                vec![
                    "07:03".to_string(),
                    "batch".to_string(),
                    "drift".to_string(),
                    "0.64".to_string(),
                ],
                vec![
                    "07:55".to_string(),
                    "edge-1".to_string(),
                    "latency".to_string(),
                    "0.77".to_string(),
                ],
                vec![
                    "08:20".to_string(),
                    "warehouse".to_string(),
                    "freshness".to_string(),
                    "0.88".to_string(),
                ],
                vec![
                    "09:02".to_string(),
                    "edge-2".to_string(),
                    "error-rate".to_string(),
                    "0.95".to_string(),
                ],
                vec![
                    "09:44".to_string(),
                    "batch".to_string(),
                    "drift".to_string(),
                    "0.58".to_string(),
                ],
            ],
        },
    ]
}

pub fn docs() -> Vec<Doc> {
    vec![
        Doc {
            slug: "the-console".to_string(),
            title: "The console".to_string(),
            blurb: "What /console actually does with a table.".to_string(),
            body: vec![
                ("TABLES".to_string(), "A table is a name, a list of column names and a list of rows, held in memory as plain Rust. The console lists every one it holds at /console, and /console/<name> is that table's own screen: its rows, its columns, and what can be asked of them.".to_string()),
                ("FILTERING AND SORTING".to_string(), "The filter box matches any cell of a row, case-insensitively. Pressing a column header sorts by that column and pressing it again reverses; numbers sort as numbers where every value in the column parses as one, and as text otherwise.".to_string()),
                ("THE CHART".to_string(), "The overview chart is recharts, off npm, fed the real row counts. The per-column chart on a table's own screen counts how often each value appears in the column you chose, which is what a profile of a column is.".to_string()),
            ],
        },
        Doc {
            slug: "getting-data-in".to_string(),
            title: "Getting data in".to_string(),
            blurb: "Two real ways, and no pretend ones.".to_string(),
            body: vec![
                ("DROP A FILE".to_string(), "Drag a .csv onto the window. The window is told the file's real path through rustx::dropped — which is the only moment a path is known, because what the page itself would receive is a File with no path — and the file is read off disk and becomes a table.".to_string()),
                ("OPEN A FILE".to_string(), "IMPORT opens the system's own file dialog through rustx::dialog and reads whatever you choose. The first line is treated as the header; every line after it is a row.".to_string()),
                ("WHAT IS NOT HERE".to_string(), "There is no database driver behind this application. It does not connect to Postgres, it does not speak a wire protocol, and it does not pretend to: the console is a GUI over tables you hand it, which is the part of database management a demonstration can get right.".to_string()),
            ],
        },
        Doc {
            slug: "getting-data-out".to_string(),
            title: "Getting data out".to_string(),
            blurb: "Export, copy, and what the file holds.".to_string(),
            body: vec![
                ("EXPORT".to_string(), "EXPORT opens the system's own save dialog and writes a real CSV: the header row, then every row of the table as it currently stands, filter and sort included. The file is then handed to the shell's recent list through rustx::os::shell::remember_recent.".to_string()),
                ("COPY".to_string(), "COPY puts the same text on the real clipboard, which is what you want when the next stop is a spreadsheet rather than a disk.".to_string()),
            ],
        },
        Doc {
            slug: "architecture".to_string(),
            title: "How this is put together".to_string(),
            blurb: "The route tree, the state, and where the work happens.".to_string(),
            body: vec![
                ("ROUTES".to_string(), "/ is the landing page; /architecture, /platform, /solutions, /developers, /docs and /company are the navigation; /contact is the call to action in the corner; /console and /console/<table> are the product.".to_string()),
                ("STATE".to_string(), "The tables live in the application's root layout rather than in a page, because they outlive the screen: importing a file on the console and reading it on a table's own screen must be the same list.".to_string()),
                ("THE HEADLINE".to_string(), "The dot-matrix type on the landing page is laid out from a 5×7 bitmap face written into src/main.rs, one round dot per lit cell. There is no such font on the machine, and a headline set in a system face would be a different design.".to_string()),
            ],
        },
    ]
}

pub fn doc_named(slug: &str) -> Option<Doc> {
    docs().into_iter().find(|doc| doc.slug == slug)
}

// ---- small helpers the pages share -------------------------------------

pub fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|letter| {
            if letter.is_ascii_alphanumeric() {
                letter
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "table".to_string()
    } else {
        slug
    }
}

/// Does every value in this column parse as a number? Decides whether
/// a column sorts as a number or as text.
pub fn all_numbers(values: &[String]) -> bool {
    !values.is_empty()
        && values
            .iter()
            .all(|value| value.trim().parse::<f64>().is_ok())
}

// ---- the 5×7 dot-matrix face ------------------------------------------

/// Seven rows of five cells, `#` where the dot is lit. Upper and lower
/// case, because the headline of the design is set in both.
pub fn glyph(letter: char) -> [&'static str; 7] {
    match letter {
        'A' => [
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'B' => [
            "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
        ],
        'C' => [
            ".####", "#....", "#....", "#....", "#....", "#....", ".####",
        ],
        'D' => [
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
        'E' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
        'F' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#....",
        ],
        'G' => [
            ".###.", "#...#", "#....", "#..##", "#...#", "#...#", ".###.",
        ],
        'H' => [
            "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'I' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####",
        ],
        'J' => [
            "..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##..",
        ],
        'K' => [
            "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
        ],
        'L' => [
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
        'M' => [
            "#...#", "##.##", "#.#.#", "#...#", "#...#", "#...#", "#...#",
        ],
        'N' => [
            "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#",
        ],
        'O' => [
            ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'P' => [
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
        'Q' => [
            ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
        ],
        'R' => [
            "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
        ],
        'S' => [
            ".####", "#....", "#....", ".###.", "....#", "....#", "####.",
        ],
        'T' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
        'U' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'V' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#..",
        ],
        'W' => [
            "#...#", "#...#", "#...#", "#...#", "#.#.#", "##.##", "#...#",
        ],
        'X' => [
            "#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#",
        ],
        'Y' => [
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
        'Z' => [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
        'a' => [
            ".....", ".....", ".###.", "....#", ".####", "#...#", ".####",
        ],
        'b' => [
            "#....", "#....", "####.", "#...#", "#...#", "#...#", "####.",
        ],
        'c' => [
            ".....", ".....", ".####", "#....", "#....", "#....", ".####",
        ],
        'd' => [
            "....#", "....#", ".####", "#...#", "#...#", "#...#", ".####",
        ],
        'e' => [
            ".....", ".....", ".###.", "#...#", "#####", "#....", ".###.",
        ],
        'f' => [
            "..##.", ".#..#", ".#...", "####.", ".#...", ".#...", ".#...",
        ],
        'g' => [
            ".....", ".####", "#...#", "#...#", ".####", "....#", ".###.",
        ],
        'h' => [
            "#....", "#....", "####.", "#...#", "#...#", "#...#", "#...#",
        ],
        'i' => [
            "..#..", ".....", ".##..", "..#..", "..#..", "..#..", ".###.",
        ],
        'j' => [
            "...#.", ".....", "..##.", "...#.", "...#.", "#..#.", ".##..",
        ],
        'k' => [
            "#....", "#....", "#..#.", "#.#..", "##...", "#.#..", "#..#.",
        ],
        'l' => [
            ".##..", "..#..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
        'm' => [
            ".....", ".....", "##.#.", "#.#.#", "#.#.#", "#.#.#", "#.#.#",
        ],
        'n' => [
            ".....", ".....", "####.", "#...#", "#...#", "#...#", "#...#",
        ],
        'o' => [
            ".....", ".....", ".###.", "#...#", "#...#", "#...#", ".###.",
        ],
        'p' => [
            ".....", "####.", "#...#", "#...#", "####.", "#....", "#....",
        ],
        'q' => [
            ".....", ".####", "#...#", "#...#", ".####", "....#", "....#",
        ],
        'r' => [
            ".....", ".....", "#.##.", "##..#", "#....", "#....", "#....",
        ],
        's' => [
            ".....", ".....", ".####", "#....", ".###.", "....#", "####.",
        ],
        't' => [
            ".#...", ".#...", "####.", ".#...", ".#...", ".#..#", "..##.",
        ],
        'u' => [
            ".....", ".....", "#...#", "#...#", "#...#", "#..##", ".##.#",
        ],
        'v' => [
            ".....", ".....", "#...#", "#...#", "#...#", ".#.#.", "..#..",
        ],
        'w' => [
            ".....", ".....", "#...#", "#...#", "#.#.#", "#.#.#", ".#.#.",
        ],
        'x' => [
            ".....", ".....", "#...#", ".#.#.", "..#..", ".#.#.", "#...#",
        ],
        'y' => [
            ".....", "#...#", "#...#", "#...#", ".####", "....#", ".###.",
        ],
        'z' => [
            ".....", ".....", "#####", "...#.", "..#..", ".#...", "#####",
        ],
        '0' => [
            ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
        ],
        '1' => [
            "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
        '2' => [
            ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
        ],
        '3' => [
            "####.", "....#", "....#", ".###.", "....#", "....#", "####.",
        ],
        '4' => [
            "#...#", "#...#", "#...#", "#####", "....#", "....#", "....#",
        ],
        '5' => [
            "#####", "#....", "####.", "....#", "....#", "#...#", ".###.",
        ],
        '6' => [
            ".###.", "#....", "#....", "####.", "#...#", "#...#", ".###.",
        ],
        '7' => [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
        '8' => [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
        '9' => [
            ".###.", "#...#", "#...#", ".####", "....#", "....#", ".###.",
        ],
        '.' => [
            ".....", ".....", ".....", ".....", ".....", ".##..", ".##..",
        ],
        ',' => [
            ".....", ".....", ".....", ".....", "..##.", "..#..", ".#...",
        ],
        _ => [
            ".....", ".....", ".....", ".....", ".....", ".....", ".....",
        ],
    }
}

/// Every lit cell of a line, as (column, row) in the line's own grid.
/// Six columns a letter, so the letters do not touch.
pub fn lit_cells(line: &str) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    for (at, letter) in line.chars().enumerate() {
        let rows = glyph(letter);
        for (y, row) in rows.iter().enumerate() {
            for (x, cell) in row.chars().enumerate() {
                if cell == '#' {
                    cells.push((at * 6 + x, y));
                }
            }
        }
    }
    cells
}

pub fn line_width(line: &str) -> usize {
    line.chars().count() * 6
}

/// The particle field under the hero: a deterministic nebula, built
/// from two filaments and a scattering around them. Returns
/// (left %, top %, size px, opacity ×100, warm) for each dot.
///
/// Deterministic rather than random, because a page that moved its own
/// stars on every render would be a different page on every render.
pub fn nebula() -> Vec<(f64, f64, f64, f64, bool)> {
    let mut dots = Vec::new();
    let mut seed: u64 = 0x2545F491_4F6CDD1D;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % 10_000) as f64 / 10_000.0
    };

    for index in 0..340 {
        let along = index as f64 / 340.0;
        // Two filaments crossing the field, the second a touch lower
        // and turning the other way.
        let strand = index % 2;
        let base = if strand == 0 {
            44.0 - 26.0 * (along * 6.0).sin()
        } else {
            56.0 + 20.0 * (along * 5.0 + 1.2).sin()
        };
        let spread = 3.0 + 13.0 * next();
        let drift = (next() - 0.5) * spread;
        let left = along * 104.0 - 2.0;
        let top = base + drift;
        let heat = next();
        let size = 1.0 + 5.0 * heat * heat;
        let bright = 0.18 + 0.72 * heat;
        dots.push((left, top, size, bright * 100.0, next() > 0.72));
    }
    dots
}
