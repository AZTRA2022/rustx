// `/datasets` — what the evals are graded against.
//
// "Add a dataset" is the real thing: a real file picker, the file read
// off disk, and the row it adds carries that file's real size, its real
// line count and its real path. "Reveal" hands the path to the system's
// file manager.

#[derive(Clone)]
struct Set {
    name: String,
    rows: u64,
    bytes: u64,
    path: String,
    mine: bool,
}

let mut data_sets: Vec<Set> = Vec::new();
let mut data_note: String = String::new();

fn Page() -> Element {
    effect((), || {
        if data_sets.is_empty() {
            data_sets = starting_sets();
        }
    });

    let rows: u64 = data_sets.iter().map(|set| set.rows).sum();
    let bytes: u64 = data_sets.iter().map(|set| set.bytes).sum();

    return <div key="datasets" id="screen-datasets" className="px-8 py-7">
        <div className="text-[13px] text-[#8b8f8b]">Workspace</div>
        <div className="mt-2 flex items-center gap-4">
            <h1 className="text-[30px] font-semibold tracking-tight">Datasets</h1>
            <span className="rounded-full bg-[#1c1e1c] px-3 py-1 text-[13px] text-[#8b8f8b]">
                {format!("{} rows · {}", thousands(rows), size(bytes))}
            </span>
            <button
                className="ml-auto rounded-xl border border-border bg-[#131413] px-3 py-2 text-[13px] text-[#9aa09a] hover:text-foreground"
                onClick={move |_| add_a_set()}
            >
                Add a dataset
            </button>
        </div>
        <p className="mt-2 text-[12px] text-[#5d615d]">{data_note.clone()}</p>

        <div className="mt-5 rounded-2xl border border-border bg-[#101110] p-5">
            <table id="sets" className="w-full text-left">
                <thead>
                    <tr className="text-[13px] text-[#8b8f8b]">
                        <th className="pb-2 font-normal">Dataset</th>
                        <th className="pb-2 font-normal">Source</th>
                        <th className="pb-2 text-right font-normal">Rows</th>
                        <th className="pb-2 text-right font-normal">Size</th>
                        <th className="pb-2 text-right font-normal"></th>
                    </tr>
                </thead>
                <tbody>
                    {data_sets.iter().map(|set| SetRow(set.clone())).collect::<Vec<_>>()}
                </tbody>
            </table>
        </div>
    </div>;
}

fn SetRow(set: Set) -> Element {
    let path = set.path.clone();
    return <tr key={set.name.clone()} className="border-t border-border text-[13px]">
        <td className="py-2.5">{set.name.clone()}</td>
        <td className="py-2.5 truncate text-[#8b8f8b]">{set.path.clone()}</td>
        <td className="py-2.5 text-right font-mono text-[#b6bbb6]">{thousands(set.rows)}</td>
        <td className="py-2.5 text-right font-mono text-[#b6bbb6]">{size(set.bytes)}</td>
        <td className="py-2.5 text-right">
            {if set.mine {
                <button
                    className="rounded-lg border border-border px-2.5 py-1 text-[12px] text-[#9aa09a] hover:text-foreground"
                    onClick={move |_| reveal(path.clone())}
                >
                    Reveal
                </button>
            } else {
                <span className="text-[12px] text-[#5d615d]">managed</span>
            }}
        </td>
    </tr>;
}

/// A real file, picked and read: the row that lands in the table is
/// this file's own size and line count, not an invented one.
fn add_a_set() {
    let ask = rustx::dialog::Ask::new("Add a dataset")
        .filter("Text or data", ["csv", "jsonl", "txt", "json"]);
    let Ok(paths) = rustx::dialog::open(ask) else {
        data_note = "the picker was closed".to_string();
        return;
    };
    let Some(path) = paths.first() else { return };
    let Ok(text) = std::fs::read_to_string(path) else {
        data_note = format!("{} could not be read as text", path.display());
        return;
    };
    let bytes = std::fs::metadata(path).map(|data| data.len()).unwrap_or(0);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "dataset".to_string());

    let mut sets = data_sets.clone();
    sets.insert(
        0,
        Set {
            rows: text.lines().count() as u64,
            path: path.display().to_string(),
            mine: true,
            bytes,
            name,
        },
    );
    data_sets = sets;
    data_note = format!("read {} from disk", path.display());
}

fn reveal(path: String) {
    match rustx::os::shell::reveal(std::path::Path::new(&path)) {
        Ok(()) => data_note = format!("{path} is open in the file manager"),
        Err(error) => data_note = format!("the file manager refused: {error}"),
    }
}

fn starting_sets() -> Vec<Set> {
    vec![
        Set { name: "support-conversations".to_string(), rows: 48_210, bytes: 94_220_000, path: "managed · vantage-storage".to_string(), mine: false },
        Set { name: "refund-decisions".to_string(), rows: 9_804, bytes: 12_400_000, path: "managed · vantage-storage".to_string(), mine: false },
        Set { name: "golden-escalations".to_string(), rows: 612, bytes: 880_000, path: "managed · vantage-storage".to_string(), mine: false },
        Set { name: "product-docs-index".to_string(), rows: 120_440, bytes: 410_000_000, path: "managed · vantage-storage".to_string(), mine: false },
    ]
}

fn size(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        return format!("{:.1} GB", bytes as f64 / 1_000_000_000.0);
    }
    if bytes >= 1_000_000 {
        return format!("{:.1} MB", bytes as f64 / 1_000_000.0);
    }
    if bytes >= 1_000 {
        return format!("{:.1} kB", bytes as f64 / 1_000.0);
    }
    format!("{bytes} B")
}

fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}
