fn Counter() -> Element {
    let mut n = 0;
    return <button onClick={|| n += 1}>{n}</button>;
}
