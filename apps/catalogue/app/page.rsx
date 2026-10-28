fn Page() -> Element {
    let title = "Catalogue";
    let items = vec!["Webcam", "Micro", "Écran"];

    return <>
        <h1>{title}</h1>
        <ul className="devices">
            {items.iter().map(|name| <li>{name}</li>).collect::<Vec<_>>()}
        </ul>
        <button id="shout" onClick={|_| clicked()}>
            {items.len()}
        </button>
    </>;
}
