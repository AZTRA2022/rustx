//! L'icône de barre système, à regarder à la main.
//!
//! `cargo run -p rustx-os --example tray_icon`
//!
//! Une icône apparaît à côté de l'horloge, avec une infobulle et une
//! notification. Clic droit : le menu. Choisissez « Quitter » — ou
//! laissez faire, l'exemple s'arrête au bout d'une minute — et l'icône
//! disparaît, parce que son jeton est lâché.
//!
//! Ce que ceci montre et qu'aucun test ne peut montrer : à quoi
//! ressemble le menu, où la bulle apparaît, et que l'icône s'en va
//! vraiment.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rustx_os::notify::What;
use rustx_os::tray::{CLICK, Icon, Item};

fn main() {
    if !rustx_os::tray::supported() {
        println!("pas de barre système ici : {}", std::env::consts::OS);
        return;
    }

    let icon = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/hybrid/assets/icon.ico");
    let tray = match Icon::new(&icon, "rustx — exemple") {
        Ok(tray) => tray,
        Err(why) => {
            println!("l'icône n'a pas été posée : {why}");
            return;
        }
    };

    if let Err(why) = tray.menu(&[
        Item::new("ouvrir", "Ouvrir la fenêtre"),
        Item::new("suivre", "Suivre la souris").checked(true),
        Item::separator(),
        Item::new("quitter", "Quitter"),
    ]) {
        println!("le menu a été refusé : {why}");
    }

    if let Err(why) = tray.notify(&What::new(
        "rustx",
        "L'icône est posée. Clic droit pour le menu.",
    )) {
        println!("la notification a été refusée : {why}");
    }

    println!("l'icône est dans la barre ({} posée(s))", rustx_os::tray::shown());
    println!("clic droit pour le menu ; « Quitter » pour finir.");

    let until = Instant::now() + Duration::from_secs(60);
    while Instant::now() < until {
        match tray.clicked().as_deref() {
            Some("quitter") => break,
            Some(CLICK) => println!("clic gauche sur l'icône"),
            Some(id) => println!("choisi : {id}"),
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }

    drop(tray);
    println!("l'icône est retirée ({} posée(s))", rustx_os::tray::shown());
}
