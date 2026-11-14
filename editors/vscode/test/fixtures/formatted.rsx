// A page already indented the way Format Document indents one.
//
// This file is the extension's own: it belongs to the test beside it and
// to nothing else, which is the whole point. The assertion it carries —
// that reindenting it changes not one character — used to be made
// against the real pages of `apps/`, and those are free to be rewritten
// by whoever owns them. A test that depends on somebody else's files is
// a test that goes red for a reason that has nothing to do with it.
//
// Every shape in here is one the indenter has to get right, and every
// one of them was a bug once:
//
//   * a generic, which must not open a level
//   * a comparison and a closure arrow, same
//   * an angle bracket inside a string, and inside a comment
//   * an attribute list written across lines, and the `>` that ends it
//   * a tag that closes itself, with its `/>` on a line of its own
//   * a method chain, written one step in
//   * a block comment, whose inside is left exactly as it was
//   * an `async move` block, a `match`, and a closure in an argument

use std::collections::HashMap;

static mut ROWS: Vec<Row> = Vec::new();

struct Row {
    name: String,
    size: u64,
}

/*
    A block comment holding a drawing:

        +------+------+
        | name | size |
        +------+------+

    whose inside is never reflowed, however it is indented.
*/

fn total(rows: &Vec<Row>) -> u64 {
    let mut sum: u64 = 0;
    // A generic, which is not a tag: `Vec<Row>` above and `HashMap<_, _>`
    // here must not push everything below them one level in.
    let seen: HashMap<String, u64> = HashMap::new();
    let _ = seen;
    for row in rows {
        if row.size < 1024 && sum > 0 {
            sum += row.size;
        } else {
            sum += 1;
        }
    }
    // A method chain is written one step in, and the `})` that ends it
    // comes back to the chain rather than out to the statement.
    let names: Vec<String> = rows
        .iter()
        .map(|row| {
            let name = row.name.clone();
            name.to_uppercase()
        })
        .filter(|name| {
            !name.is_empty()
                && !name.starts_with('.')
        })
        .collect();
    let _ = names;
    sum
}

fn Page() -> Element {
    let mut note = String::new();
    // An angle bracket inside a string opens nothing: `"<p>"` here is
    // text, and taking it for a tag left every line below it one level
    // in for the rest of the file.
    let sample = "<p>not a tag</p>";
    let _ = sample;
    // And a closure arrow is not a tag either.
    let bump = move |n: u32| n + 1;
    let _ = bump(1);

    match total(&Vec::new()) {
        0 => note = String::from("nothing yet"),
        _ => note = format!("{} bytes", 1),
    }

    async move {
        let read = tokio::task::spawn_blocking(|| std::fs::read_to_string("rows.csv").ok()).await;
        let _ = read;
    };

    return <main class="p-8">
        <h1>Rows</h1>
        <section
            id="rows"
            class="grid gap-4"
            data-note={note.clone()}
        >
            <p>{note.clone()}</p>
            <br />
            <Card
                title="Total"
                value={total(&Vec::new())}
            />
            <Card title="One line" value={1} />
            <ul>
                <li>one</li>
                <li>two</li>
            </ul>
        </section>
        <footer>
            <small>fin</small>
        </footer>
    </main>;
}
