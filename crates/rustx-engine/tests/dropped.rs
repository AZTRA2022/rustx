//! Les fichiers lâchés sur la fenêtre, avec leur vrai chemin.
//!
//! Ce que le web donne — un `File` du `DataTransfer` — n'a pas de
//! chemin : impossible de le rouvrir, de le passer à une bibliothèque
//! Rust, d'enregistrer à côté. Seul le gestionnaire natif de Chromium
//! connaît le chemin, et il vit dans le processus de la fenêtre.
//!
//! Ce qui se prouve ici : les chemins traversent intacts — accents
//! compris — les crochets sont appelés dans l'ordre, du texte lâché
//! n'est pas pris pour un fichier, et une page qui préfère lire plutôt
//! que s'abonner lit la même chose. Le geste lui-même demande une main :
//! `cargo run -p hybrid` puis un fichier tiré depuis l'explorateur.

use std::path::PathBuf;

use rustx_engine::{Value, bridge, dropped};

fn speaking_cef() {
    rustx_engine::declare_api_version();
}

/// Ce que le côté fenêtre envoie, remis comme Chromium le remet.
fn arriving(paths: &[&str]) {
    let values: Vec<Value> = paths
        .iter()
        .map(|path| Value::String((*path).to_string()))
        .collect();
    let message = bridge::saying(dropped::WHAT, &values).expect("un message de Chromium");
    assert!(bridge::told(&message), "personne n'écoutait les fichiers");
    // Et le lâcher, que la page annonce : les chemins arrivent quand le
    // curseur entre dans la fenêtre, le lâcher vient après.
    dropped::landed();
}

#[test]
fn a_dropped_file_arrives_with_its_real_path() {
    speaking_cef();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let keep = seen.clone();
    dropped::on_files(move |paths| keep.borrow_mut().push(paths));

    // Un chemin accentué, et un espace : les deux cassent une
    // conversion écrite à la main.
    arriving(&[
        "C:\\Users\\aztra\\Images\\éléphant rose.png",
        "C:\\Users\\aztra\\Musique\\été.mp3",
    ]);

    let seen = seen.borrow();
    assert_eq!(seen.len(), 1, "un dépôt, un appel");
    assert_eq!(
        seen[0],
        vec![
            PathBuf::from("C:\\Users\\aztra\\Images\\éléphant rose.png"),
            PathBuf::from("C:\\Users\\aztra\\Musique\\été.mp3"),
        ]
    );
}

#[test]
fn a_page_that_prefers_to_read_reads_the_same_thing() {
    speaking_cef();
    // Branché comme le processus de rendu le branche, avant qu'un
    // dépôt arrive : une page qui ne lit qu'au premier battement
    // manquerait sinon ce qui a été lâché avant lui.
    dropped::ready();
    arriving(&["C:\\Travail\\dossier"]);
    // Lu une fois : ce qui a été lu n'est plus en attente, sinon un
    // battement de la page rouvrirait le même dossier sans fin.
    assert_eq!(dropped::files(), vec![PathBuf::from("C:\\Travail\\dossier")]);
    assert!(dropped::files().is_empty());
}

#[test]
fn dropped_text_is_not_mistaken_for_a_file() {
    speaking_cef();
    let called = std::rc::Rc::new(std::cell::Cell::new(0));
    let keep = called.clone();
    dropped::on_files(move |_| keep.set(keep.get() + 1));

    // Du texte lâché sur la fenêtre : le gestionnaire natif ne donne
    // aucun nom de fichier, et un dépôt sans fichier n'est pas un
    // dépôt de fichiers.
    let before = called.get();
    arriving(&[]);
    assert_eq!(called.get(), before, "un dépôt sans fichier a appelé le crochet");
    assert!(dropped::files().is_empty());
}

#[test]
fn what_the_window_sends_is_what_the_handler_read() {
    // La conversion du côté fenêtre : une liste de noms de Chromium
    // devient des valeurs, et une liste vide ne devient rien du tout.
    assert_eq!(
        dropped::carried(&["a.png".to_string(), "b.png".to_string()]),
        vec![
            Value::String("a.png".to_string()),
            Value::String("b.png".to_string())
        ]
    );
    assert!(dropped::carried(&[]).is_empty());
}

#[test]
fn a_dropped_file_does_not_replace_the_application() {
    // Trouvé en lâchant un `.rsx` sur une vraie fenêtre : Chromium
    // faisait ce qu'un navigateur fait d'un fichier lâché sur une page
    // — il y naviguait — et l'application avait disparu, remplacée par
    // le texte du fichier.
    //
    // Ce que le moteur met dans chaque page est donc ce qui reprend le
    // lâcher à Chromium. Épinglé ici parce que c'est une ligne qu'on
    // peut supprimer sans qu'aucun autre test ne s'en aperçoive.
    assert!(
        dropped::LISTENER.contains("preventDefault"),
        "got {}",
        dropped::LISTENER
    );
    assert!(dropped::LISTENER.contains("'dragover'"), "le survol aussi : sans lui, le lâcher n'est même pas proposé");
    assert!(dropped::LISTENER.contains("__rustx_dropped"));
}
