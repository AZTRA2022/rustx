let mut order = vec!["a".to_string(), "b".to_string(), "c".to_string()];

fn Page() -> Element {
    return <div>
        <div id="first">{Counter()}</div>
        <div id="second">{Counter()}</div>

        <ul id="rows">
            {order.iter().map(|key| <li key={key.clone()}>{Counter()}</li>).collect::<Vec<_>>()}
        </ul>

        <button id="rotate" onClick={move |_| order.rotate_right(1)}>rotate</button>
    </div>;
}

/// One `let mut n = 0;`, as many counters as there are places it is
/// written — and as many as there are keys in a list.
fn Counter() -> Element {
    let mut n = 0;
    return <button onClick={move |_| n += 1}>{n}</button>;
}
