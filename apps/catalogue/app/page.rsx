// Le catalogue : une liste, un bouton, et une zone où lâcher des
// fichiers.
//
// Le dépôt est là pour une raison précise : ce qu'une page web reçoit
// quand on lâche un fichier dessus est un objet sans chemin. Ce que
// `rustx::dropped` donne est le vrai chemin, celui qu'on peut rouvrir
// demain. La page le montre tel quel.

let mut clicked = false;
let mut dropped_here: String = String::new();

fn Page() -> Element {
    let title = "Catalogue";
    let items = vec!["Webcam", "Micro", "Écran"];

    // Posé une fois, et non lu à chaque battement : `on_files` est
    // appelé quand la fenêtre reçoit quelque chose, et écrire l'état
    // depuis là est permis — ce qui est interdit, c'est de l'écrire
    // pendant qu'on dessine.
    effect((), || {
        rustx::dropped::on_files(move |paths| {
            if let Some(path) = paths.first() {
                dropped_here = path.display().to_string();
            }
        });
    });

    return <>
        <h1>{title}</h1>
        <ul className="devices">
            {items.iter().map(|name| <li>{name}</li>).collect::<Vec<_>>()}
        </ul>
        <button id="shout" onClick={move |_| clicked = true}>
            {items.len()}
        </button>
        <div id="drop-zone" style={{padding: "2rem", border: "2px dashed #888", marginTop: "1rem"}}>
            <p>Lâchez un fichier ici.</p>
            <p id="dropped">{dropped_here}</p>
        </div>
    </>;
}
