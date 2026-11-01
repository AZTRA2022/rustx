//! Une deuxième fenêtre : son nom, sa route, et le message qui la demande.
//!
//! Ce qui se prouve ici est tout ce qui ne demande pas deux vrais
//! processus : ce qu'une page écrit, ce que le message porte, et la
//! route qu'une URL dit. Le reste — la fenêtre qui apparaît, la
//! première qu'on ferme sans arrêter l'application — est une mesure à
//! la main, et elle est écrite dans le plan.

use rustx_engine::window::{self, What};

#[test]
fn a_window_is_named_titled_and_sized_by_what_it_was_given() {
    let asked = What::new("réglages");
    assert_eq!(asked.name, "réglages");
    // Le nom sert de titre jusqu'à ce qu'on en donne un : une fenêtre
    // sans titre est une fenêtre qu'on ne retrouve pas dans la barre
    // des tâches.
    assert_eq!(asked.title, "réglages");
    assert_eq!(asked.at, "/");

    let asked = asked.titled("Réglages").size(500, 400).at("/settings");
    assert_eq!(asked.title, "Réglages");
    assert_eq!((asked.width, asked.height), (500, 400));
    assert_eq!(asked.at, "/settings");
}

#[test]
fn the_route_a_window_opens_on_travels_in_the_fragment() {
    assert_eq!(
        window::page_in("file:///C:/app/assets/index.html#/device/7").as_deref(),
        Some("/device/7"),
        "c'est par là qu'une deuxième fenêtre apprend sa route"
    );
}

#[test]
fn a_document_without_a_fragment_asks_for_no_particular_route() {
    // La première fenêtre : elle part de la route de départ de
    // l'application, pas d'une route qu'une URL lui dicte.
    assert_eq!(window::page_in("file:///C:/app/assets/index.html"), None);
    assert_eq!(window::page_in("file:///C:/app/assets/index.html#"), None);
    assert_eq!(window::page_in("file:///C:/app/assets/index.html#  "), None);
}

#[test]
fn a_route_is_a_route_however_it_was_written() {
    // Un développeur qui écrit `at("settings")` a voulu `/settings`.
    assert_eq!(
        window::page_in("file:///index.html#settings").as_deref(),
        Some("/settings")
    );
}

#[test]
fn a_window_asked_for_outside_a_page_is_refused_in_words() {
    // Ce test tourne dans un processus ordinaire : pas de contexte, donc
    // pas de frame, donc rien pour porter la demande. Le refus doit le
    // dire, parce que l'erreur qu'un développeur fera est d'appeler
    // `window::open` depuis `main` plutôt que depuis la page.
    let refused = window::open(What::new("seconde")).expect_err("il n'y a pas de page ici");
    let said = refused.to_string();
    assert!(said.contains("page"), "got {said}");
}

#[test]
fn nothing_is_open_in_a_process_that_has_no_window() {
    assert_eq!(window::how_many_are_open(), 0);
}
