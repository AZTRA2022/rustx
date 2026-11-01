//! The proof of plan 2: a page written in `.rsx`, on screen.

mod common;

use std::sync::OnceLock;

use common::{Report, run_catalogue_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(run_catalogue_scenario)
}

#[test]
fn the_page_written_in_rsx_is_on_screen() {
    assert_eq!(
        report().get("html"),
        concat!(
            "<h1>Catalogue</h1>",
            "<ul class=\"devices\"><li>Webcam</li><li>Micro</li><li>Écran</li></ul>",
            "<button id=\"shout\">3</button>",
            "<div id=\"drop-zone\" style=\"padding: 2rem; border: 2px dashed #888; ",
            "margin-top: 1rem\"><p>Lâchez un fichier ici.</p><p id=\"dropped\"></p></div>",
        ),
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn clicking_the_button_runs_the_rust_written_in_the_rsx() {
    assert_eq!(
        report().get("clicked"),
        "true",
        "the closure written in the .rsx never ran:\n{}",
        report().raw()
    );
}

#[test]
fn the_engine_takes_the_drop_back_from_the_browser() {
    // La moitié du glisser-déposer qui se prouve sans main humaine.
    //
    // Un navigateur, par défaut, *navigue* vers le fichier qu'on lâche
    // sur une page : mesuré en lâchant un `.rsx` sur cette fenêtre,
    // l'application avait disparu, remplacée par le texte du fichier.
    // Le moteur reprend donc le lâcher à Chromium, et c'est ce que ces
    // trois lignes épinglent.
    assert_eq!(
        report().get("drop_native"),
        "function",
        "le moteur n'a pas posé la fonction que son écouteur appelle:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("drop_prevented"),
        "true",
        "le lâcher est resté au navigateur, qui va naviguer vers le fichier:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("dragover_prevented"),
        "true",
        "sans le survol, Windows ne propose même pas le dépôt:\n{}",
        report().raw()
    );
    // Les chemins, eux, demandent un vrai geste : seul le gestionnaire
    // natif de Chromium les connaît, et aucun événement fabriqué en
    // JavaScript n'en porte. C'est `cargo run -p catalogue`, un fichier
    // tiré depuis l'explorateur, et le chemin sous la zone.
}
