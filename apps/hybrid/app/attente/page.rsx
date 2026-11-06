// A page that waits, and a window that does not.
//
// Three things are proved here and nowhere else in this application:
//
//   - `.await` inside a `task(async { … })`, on a timer that belongs to
//     the runtime and not to the compositor, so it fires on a machine
//     whose window produces no frames of its own;
//   - state written from inside that future — a `let mut` at the top of
//     this file, written after an `await` exactly as it would be written
//     from an `onClick`;
//   - the work stopping when the page leaves, because the token is a
//     cell of this component and a cell goes when the component goes.
//
// A component itself is never `async`: it is asked for the screen as it
// is now, so it cannot wait. That is what keeps the window alive while
// all of this happens.

let mut rows = Vec::<u32>::new();
let mut burst = 0;
let mut ticks = 0;

fn Page() -> Element {
    // How many times this page has been drawn. A plain counter in a kept
    // cell rather than state: state written during a render would ask
    // for another render, and this only has to be read.
    let drawn = keep(std::cell::Cell::new(0u32));
    drawn.with(|count| count.set(count.get() + 1));

    // Three awaits, and a write after each one.
    task(async move {
        for step in 1..=3u32 {
            rustx::time::sleep(std::time::Duration::from_millis(40)).await;
            rows.push(step);
        }
    });

    // A thousand writes with nothing between them. They are posted on
    // the mailbox and applied together by the frame that drains it, so
    // the screen is drawn once for all of them — the same contract a
    // worker thread has had since the beginning.
    task(async move {
        for _ in 0..1_000 {
            burst += 1;
        }
    });

    // And one that never ends, to prove that leaving stops it. Nothing
    // here cancels it by name.
    task(async move {
        loop {
            rustx::time::sleep(std::time::Duration::from_millis(20)).await;
            ticks += 1;
        }
    });

    let written: Vec<Element> = rows
        .iter()
        .map(|step| {
            Element::tag("li")
                .attribute("class", rustx::AttributeValue::Text("row".to_string()))
                .child(Element::Text(format!("étape {step}")))
                .build()
        })
        .collect();

    return <section id="attente" className="mx-auto max-w-2xl px-6 py-16 text-zinc-100">
        <h1 id="attente-title" className="text-3xl font-semibold tracking-tight">
            Une page qui attend
        </h1>

        <p className="mt-2 text-sm text-zinc-400">
            Trois `await`, mille écritures, et un travail qui s'arrête en partant.
        </p>

        <ul id="rows" className="mt-6 space-y-1 text-sm">{written}</ul>

        <dl className="mt-8 grid grid-cols-3 gap-4 text-sm">
            <div>
                <dt className="text-zinc-500">écrites</dt>
                <dd id="rows-count" className="text-lg">{rows.len()}</dd>
            </div>
            <div>
                <dt className="text-zinc-500">en rafale</dt>
                <dd id="burst" className="text-lg">{burst}</dd>
            </div>
            <div>
                <dt className="text-zinc-500">dessins</dt>
                <dd id="drawn" className="text-lg">{drawn.with(|count| count.get())}</dd>
            </div>
        </dl>

        <button
            id="attente-leave"
            className="mt-10 rounded-md bg-zinc-800 px-4 py-2 text-sm"
            onClick={move |_| { rustx::router::navigate("/merci"); }}
        >Quitter</button>
    </section>;
}
