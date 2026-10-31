//! Les dialogues de fichiers natifs.
//!
//! Un dialogue modal attend une main humaine, donc ce qui est prouvé
//! ici est tout ce qui l'entoure : la conversion des filtres, les mots
//! des refus, et la forme de ce qu'on demande. Le dialogue lui-même se
//! regarde avec `cargo run -p rustx-os --example choose_a_file`.

use rustx_os::dialog::{self, Ask, DialogError};

#[test]
fn the_filters_become_patterns_every_system_reads() {
    let asked = vec![
        (
            "Images".to_string(),
            vec!["png".to_string(), ".jpg".to_string(), "*.gif".to_string()],
        ),
        ("Texte".to_string(), vec!["txt".to_string()]),
    ];
    assert_eq!(
        dialog::patterns_for(&asked),
        vec![
            ("Images".to_string(), "*.png;*.jpg;*.gif".to_string()),
            ("Texte".to_string(), "*.txt".to_string()),
        ],
        "le motif est le même quelle que soit la façon d'écrire l'extension"
    );
    assert!(dialog::patterns_for(&[]).is_empty());
}

#[test]
fn what_is_asked_is_what_is_written() {
    // `Ask` est écrit à la main dans une page, donc il se lit.
    let ask = Ask::new("Choisir une image")
        .filter("Images", ["png"])
        .starting_in("C:/Users")
        .named("sans-titre.png")
        .many();
    assert_eq!(ask.title, "Choisir une image");
    assert_eq!(ask.filters.len(), 1);
    assert_eq!(ask.start_in, Some(std::path::PathBuf::from("C:/Users")));
    assert_eq!(ask.named.as_deref(), Some("sans-titre.png"));
    assert!(ask.many);
    // Et rien n'est demandé par défaut.
    let plain = Ask::new("Ouvrir");
    assert!(plain.filters.is_empty() && plain.start_in.is_none() && !plain.many);
}

#[test]
fn a_cancelled_dialog_is_cancelled_and_not_a_failure() {
    // Nommé, et dit en français clair : une application qui journalise
    // une panne à chaque fois qu'un utilisateur ferme un dialogue est
    // une application dont les journaux ne servent plus à rien.
    let said = DialogError::Cancelled.to_string();
    assert!(said.contains("fermé"), "got {said}");
    assert!(!said.contains("erreur"), "got {said}");
}

#[test]
fn a_platform_that_cannot_do_it_says_which_file_to_write() {
    // Le refus d'une plateforme non écrite nomme la plateforme *et* le
    // fichier : c'est une liste de travail, pas une excuse.
    let said = DialogError::Unsupported("le dialogue de fichiers").to_string();
    assert!(said.contains("le dialogue de fichiers"), "got {said}");
    assert!(said.contains(std::env::consts::OS), "got {said}");
    assert!(said.contains("dialog.rs"), "got {said}");
}

#[test]
fn this_platform_says_whether_it_has_one() {
    // Windows en a un ; ailleurs, `false` tant que le fichier n'est pas
    // écrit — ce qu'une page lit pour ne pas dessiner un bouton qui
    // refuserait.
    assert_eq!(dialog::supported(), cfg!(windows));
}
