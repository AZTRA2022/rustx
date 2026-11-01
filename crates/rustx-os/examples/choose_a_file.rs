//! Le dialogue de fichiers, à regarder à la main.
//!
//! `cargo run -p rustx-os --example choose_a_file`
//! `cargo run -p rustx-os --example choose_a_file -- save`
//! `cargo run -p rustx-os --example choose_a_file -- folder`
//!
//! Le dialogue du système s'ouvre, vous choisissez ou vous fermez, et le
//! terminal dit ce qui est arrivé. Aucune fenêtre d'application, aucun
//! Chromium : c'est précisément ce que le dialogue natif permet, et
//! c'est pour cela qu'il a été préféré à celui du navigateur.
//!
//! Pourquoi un exemple et non un test : un dialogue modal attend une
//! main humaine. Ce qu'une suite peut prouver sans personne devant
//! l'écran est dans `tests/dialog.rs`.

use rustx_os::dialog::{self, Ask};

fn main() {
    let what = std::env::args().nth(1).unwrap_or_default();
    let ask = Ask::new("rustx — choisir")
        .filter("Images", ["png", "jpg", "jpeg"])
        .filter("Texte", ["txt", "md"])
        .named("sans-titre.png")
        .many();

    match what.as_str() {
        "save" => match dialog::save(ask) {
            Ok(path) => println!("ENREGISTRER {}", path.display()),
            Err(why) => println!("RIEN : {why}"),
        },
        "folder" => match dialog::folder(ask) {
            Ok(path) => println!("DOSSIER {}", path.display()),
            Err(why) => println!("RIEN : {why}"),
        },
        _ => match dialog::open(ask) {
            Ok(paths) => {
                println!("CHOISI {} fichier(s) :", paths.len());
                for path in paths {
                    println!("  {}", path.display());
                }
            }
            Err(why) => println!("RIEN : {why}"),
        },
    }
}
