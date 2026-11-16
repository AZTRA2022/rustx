//! Les écrans : combien, où, de quelle taille, à quelle densité.
//!
//! Ce qu'une page ne sait pas : `window.screen` ne parle que de l'écran
//! où la fenêtre se trouve, et ne dit rien des autres. Une application
//! qui ouvre sa deuxième fenêtre sur le deuxième moniteur, ou qui place
//! un panneau là où la barre des tâches n'est pas, a besoin de la liste.
//!
//! # Deux rectangles par écran
//!
//! `bounds` est l'écran entier. `work` est ce qu'il en reste quand on
//! enlève les barres que le bureau y a posées — la barre des tâches, le
//! Dock, un panneau. Une fenêtre maximisée occupe `work`, pas `bounds`,
//! et c'est la différence qu'il faut pour ne pas la placer sous une
//! barre.
//!
//! Les coordonnées sont celles du bureau, un seul repère pour tous les
//! moniteurs : un écran à gauche du principal a un `x` négatif, et c'est
//! une coordonnée comme une autre.

use crate::error::OsError;

/// Un rectangle, en coordonnées de bureau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Area {
    /// Depuis deux coins — ce que les trois systèmes rendent.
    pub fn between(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }

    /// Si ce point est dedans, bord gauche et haut compris.
    pub fn holds(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

/// Un écran.
#[derive(Debug, Clone, PartialEq)]
pub struct Screen {
    /// Le nom que le système lui donne : `\\.\DISPLAY1`, `DP-2`, ou le
    /// numéro de l'écran faute de nom.
    pub name: String,
    /// L'écran entier.
    pub bounds: Area,
    /// Ce qu'il en reste sous les barres du bureau.
    pub work: Area,
    /// Son facteur d'échelle : `1.0` à 96 dpi, `2.0` sur un Retina.
    pub scale: f64,
    /// Celui où le bureau commence, et où les boîtes de dialogue
    /// s'ouvrent.
    pub primary: bool,
}

/// Tous les écrans, le principal en premier.
///
/// L'ordre est voulu : une application qui ne veut qu'un écran veut
/// celui-là, et `screens()[0]` doit être lui sans avoir à chercher.
pub fn screens() -> Result<Vec<Screen>, OsError> {
    let mut all = crate::platform::screen::screens()?;
    if all.is_empty() {
        return Err(OsError::Platform(
            "the system reported no screen at all".to_string(),
        ));
    }
    all.sort_by_key(|screen| !screen.primary);
    Ok(all)
}

/// Celui où le bureau commence.
pub fn primary() -> Result<Screen, OsError> {
    screens()?
        .into_iter()
        .find(|screen| screen.primary)
        // Aucun ne se dit principal : le premier, parce qu'une liste non
        // vide vaut mieux qu'un refus pour une question qui a toujours
        // une réponse à l'écran.
        .or_else(|| screens().ok().and_then(|all| all.into_iter().next()))
        .ok_or(OsError::Unsupported("the list of screens"))
}

/// Celui qui contient ce point, ou le principal si aucun ne le contient
/// — ce qui arrive entre deux moniteurs de hauteurs différentes.
pub fn holding(x: i32, y: i32) -> Result<Screen, OsError> {
    let all = screens()?;
    Ok(all
        .iter()
        .find(|screen| screen.bounds.holds(x, y))
        .cloned()
        .unwrap_or_else(|| all[0].clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_area_is_two_corners_and_knows_what_it_holds() {
        let left = Area::between(-1920, 0, 0, 1080);
        assert_eq!(left.width, 1920);
        assert_eq!(left.height, 1080);
        assert_eq!(
            left.x, -1920,
            "a monitor left of the primary has a negative x"
        );
        assert!(left.holds(-1920, 0), "the top-left corner is inside");
        assert!(!left.holds(0, 0), "the right edge is not");
        assert!(!left.holds(-1921, 500));
    }

    #[test]
    fn this_machine_lists_its_screens_or_refuses_by_name() {
        match screens() {
            Ok(all) => {
                assert!(!all.is_empty());
                assert!(
                    all[0].primary || all.iter().all(|screen| !screen.primary),
                    "the primary screen comes first"
                );
                for screen in &all {
                    assert!(
                        screen.bounds.width > 0 && screen.bounds.height > 0,
                        "{screen:?}"
                    );
                    assert!(screen.scale > 0.0, "{screen:?}");
                    assert!(
                        screen.work.width <= screen.bounds.width
                            && screen.work.height <= screen.bounds.height,
                        "the work area fits inside the screen: {screen:?}"
                    );
                }
            }
            Err(refused) => assert!(!refused.to_string().is_empty()),
        }
    }
}
