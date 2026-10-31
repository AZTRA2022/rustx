//! Un raccourci global, à essayer à la main.
//!
//! `cargo run -p rustx-os --example a_global_shortcut`
//! `cargo run -p rustx-os --example a_global_shortcut -- "Ctrl+Alt+K"`
//!
//! Appuyez sur la combinaison — depuis ce terminal, depuis un
//! navigateur, depuis n'importe où : c'est ce que « global » veut dire.
//! Chaque appui est compté et écrit. Au bout d'une minute, ou sur
//! `Ctrl+C`, le jeton est lâché et les touches sont rendues.

use std::time::{Duration, Instant};

use rustx_os::shortcut::{self, Shortcut};

fn main() {
    let keys = std::env::args().nth(1).unwrap_or_else(|| "Ctrl+Shift+R".to_string());

    if !shortcut::supported() {
        println!("pas de raccourcis globaux ici : {}", std::env::consts::OS);
        return;
    }

    match shortcut::parse(&keys) {
        Ok((modifiers, key)) => println!("lu : modificateurs {modifiers:#06x}, touche {key:#04x}"),
        Err(why) => {
            println!("{why}");
            return;
        }
    }

    let held = match Shortcut::listen(&keys) {
        Ok(held) => held,
        Err(why) => {
            println!("{why}");
            return;
        }
    };

    println!("{keys} est à nous. Appuyez dessus, d'où vous voulez.");
    let until = Instant::now() + Duration::from_secs(60);
    let mut total = 0;
    while Instant::now() < until {
        let since = held.pressed();
        if since > 0 {
            total += since;
            println!("appuyé {since} fois ({total} en tout)");
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    drop(held);
    println!("{keys} est rendu.");
}
