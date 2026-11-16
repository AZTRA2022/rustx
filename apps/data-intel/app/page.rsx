// The landing page.
//
// The headline is not a font: the design is set in a dot-matrix face
// and there is none on this machine, so the two lines are laid out
// cell by cell from the 5×7 table in `src/main.rs` — one round dot per
// lit cell.
//
// The field under the rule is the same idea with the dots scattered:
// three hundred and forty of them along two filaments, every position
// computed once in `nebula()` so the page is the same page on every
// render rather than a different sky each frame.

let mut tables: Vec<Dataset> = Vec::new();
let mut imported: u32 = 0;

fn Page() -> Element {
    let rows: usize = tables.iter().map(|table| table.rows.len()).sum();

    return <div key="home" id="screen-home">
        <div className="border-b border-border px-6 pb-12 pt-16">
            <div className="mx-auto w-fit">
                {Line("The Intelligence layer".to_string(), 0, 7)}
                <div className="h-[18px]"></div>
                {Line("for your data".to_string(), 1, 7)}
            </div>

            <p className="mx-auto mt-10 max-w-[430px] text-center text-[11px] leading-[1.9] text-muted-foreground">
                Our layer aggregates signals, removes noise, and delivers only what is actionable
                at scale and in real time.
            </p>

            <div className="mt-8 flex items-center justify-center gap-3">
                <button
                    className="slab rounded-[5px] px-6 py-[9px] text-[12px]"
                    onClick={move |_| rustx::router::navigate("/console".to_string())}
                >
                    Get started
                </button>
                <button
                    className="glossy rounded-[5px] px-6 py-[9px] text-[12px]"
                    onClick={move |_| rustx::router::navigate("/architecture".to_string())}
                >
                    Learn more
                </button>
            </div>
        </div>

        {Field(rows)}

        <div className="mx-auto grid max-w-[1000px] grid-cols-3 gap-4 px-6 py-12">
            {Card(
                "AGGREGATE".to_string(),
                "Every signal that reaches the layer lands in one place, keyed the same way, whatever it came from.".to_string(),
                "/architecture".to_string(),
            )}
            {Card(
                "REMOVE NOISE".to_string(),
                "A row that tells you nothing is a row you pay to store twice. The layer keeps what cleared the floor.".to_string(),
                "/platform".to_string(),
            )}
            {Card(
                "DELIVER".to_string(),
                "What is left is a table, and a table is something a person can open. The console is at /console.".to_string(),
                "/console".to_string(),
            )}
        </div>
    </div>;
}

// ---- the dot-matrix headline -----------------------------------------

/// One line of the headline. `cell` is the pitch; the dot itself is
/// a touch smaller, which is what makes it read as a matrix rather
/// than as a block.
fn Line(text: String, line: usize, cell: usize) -> Element {
    let frame = format!(
        "position:relative;height:{}px;width:{}px",
        7 * cell,
        line_width(&text) * cell
    );
    let cells = lit_cells(&text);

    return <div key={line.to_string()} style={frame}>
        {cells
            .iter()
            .map(|at| Dot(format!("{line}-{}-{}", at.0, at.1), at.0, at.1, cell))
            .collect::<Vec<_>>()}
    </div>;
}

fn Dot(name: String, column: usize, row: usize, cell: usize) -> Element {
    let size = cell.saturating_sub(2).max(2);
    let paint = format!(
        "position:absolute;left:{}px;top:{}px;width:{size}px;height:{size}px;border-radius:50%;background:#e8e8e8",
        column * cell + 1,
        row * cell + 1
    );
    return <span key={name} style={paint}></span>;
}

// ---- the particle field ----------------------------------------------

fn Field(rows: usize) -> Element {
    return <div className="relative h-[330px] overflow-hidden border-b border-border bg-black">
        {nebula()
            .iter()
            .enumerate()
            .map(|(index, dot)| Speck(index, dot.0, dot.1, dot.2, dot.3, dot.4))
            .collect::<Vec<_>>()}
        <span className="absolute left-6 top-5 text-[9px] tracking-[0.18em] text-faint">
            SIGNAL FIELD · LIVE
        </span>
        <span className="absolute bottom-5 right-6 text-[9px] tracking-[0.18em] text-faint">
            {format!("{} ROWS UNDER MANAGEMENT · {} TABLES", rows, tables.len())}
        </span>
    </div>;
}

fn Speck(index: usize, left: f64, top: f64, size: f64, bright: f64, warm: bool) -> Element {
    let colour = if warm { "217,138,74" } else { "214,226,238" };
    let paint = format!(
        "position:absolute;left:{left:.2}%;top:{top:.2}%;width:{size:.1}px;height:{size:.1}px;border-radius:50%;background:rgba({colour},{:.2});box-shadow:0 0 {:.0}px rgba({colour},{:.2})",
        bright / 100.0,
        size * 2.4,
        bright / 240.0
    );
    return <span key={index.to_string()} style={paint}></span>;
}

// ---- the three cards under it ----------------------------------------

fn Card(name: String, text: String, route: String) -> Element {
    let going = route.clone();
    return <button
        key={name.clone()}
        className="rounded-[6px] border border-border bg-card p-5 text-left hover:border-[#3a3a3a]"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="block text-[10px] tracking-[0.16em] text-faint">{name}</span>
        <span className="mt-3 block text-[12px] leading-[1.75] text-foreground">{text}</span>
        <span className="mt-4 block text-[10px] tracking-[0.12em] text-muted-foreground">READ ON →</span>
    </button>;
}
