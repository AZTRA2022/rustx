//! A value a component waits for without waiting.
//!
//! A component is asked for the screen as it is now, so it cannot stop
//! to wait. What it can do is ask once, draw nothing, and be drawn
//! again when the answer arrives — which is what `let x = f().await;`
//! in a page turns into. No window is needed to prove it: a cell, a
//! frame that drains the mailbox, and a future.

use std::time::{Duration, Instant};

use rustx_dom::state::{InstanceId, Scope, Slot, apply_remote, open_remote};
use rustx_engine::task;

macro_rules! here {
    () => {
        Slot {
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

#[test]
fn the_first_render_gets_nothing_and_a_later_one_gets_the_value() {
    open_remote();
    let _scope = Scope::enter(InstanceId(9_101));

    // The two places the compiler writes: where the `let` is, and where
    // the `.await` was. The same two on every render, which is what
    // makes the future be asked for once.
    let value = here!();
    let spawn = here!();
    let render = || task::awaited::<u32, _>(value, spawn, "/", || async { 41u32 + 1 });

    assert_eq!(render(), None, "the render waited for the future");

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut got = None;
    while Instant::now() < deadline {
        // What a frame does: drain what the futures posted, then draw.
        apply_remote();
        got = render();
        if got.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(got, Some(42), "the value never reached the cell");
    assert_eq!(render(), Some(42), "it did not stay there");
}

#[test]
fn the_same_question_is_asked_again_when_the_screen_is_another_one() {
    // `/depots/rust-lang/rust` et `/depots/tokio-rs/tokio` sont la même
    // route, donc le même composant au même endroit, donc la même
    // cellule : une question posée une fois par cellule est une question
    // posée une fois pour les deux dépôts, et le second montrait la
    // réponse du premier. La seule façon de le savoir était de l'avoir
    // vu, et la seule parade un `with_key` écrit à la main.
    open_remote();
    let _scope = Scope::enter(InstanceId(9_103));

    let value = here!();
    let spawn = here!();
    // Ce que la question rend dépend de l'écran, comme une vraie : elle
    // lit un paramètre de la route.
    let render = |screen: &str| {
        let answer = screen.to_string();
        task::awaited::<String, _>(value, spawn, screen, || async move { answer })
    };

    let settled = |screen: &'static str| {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut got = None;
        while Instant::now() < deadline {
            apply_remote();
            got = render(screen);
            if got.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        got
    };

    assert_eq!(
        settled("/depots/rust-lang/rust").as_deref(),
        Some("/depots/rust-lang/rust")
    );
    assert_eq!(
        settled("/depots/tokio-rs/tokio").as_deref(),
        Some("/depots/tokio-rs/tokio"),
        "le second écran a montré la réponse du premier"
    );
    // Et revenir au premier ne repose pas la question pour rien : sa
    // réponse est encore là, sous sa clé.
    assert_eq!(
        render("/depots/rust-lang/rust").as_deref(),
        Some("/depots/rust-lang/rust"),
        "la réponse du premier écran a été perdue"
    );
}
