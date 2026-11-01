//! Une seule instance, et ce que la deuxième lui passe.
//!
//! Un logiciel qu'on relance alors qu'il tourne déjà ne doit pas
//! s'ouvrir deux fois : il doit réveiller celui qui tourne, et lui
//! passer ce qu'on lui a donné — le fichier qu'on vient de
//! double-cliquer, par exemple.
//!
//! Tout se prouve ici dans un seul processus : le nom est réservé
//! auprès du système, donc un deuxième `only_one` dans le même
//! processus est refusé exactement comme le serait un deuxième
//! programme, et ce qu'une deuxième instance passe est remis à la
//! fenêtre invisible de la première, qui est la nôtre.

use rustx_os::instance::{self, Only};

/// Un nom qui n'est celui de personne d'autre pendant les tests.
fn a_name() -> String {
    format!("rustx-test-{}", std::process::id())
}

#[test]
fn a_second_instance_is_refused_by_name() {
    if !instance::supported() {
        return;
    }
    let name = a_name();
    let first = Only::one(&name).expect("personne ne tient ce nom");
    let second = Only::one(&name);
    let said = second
        .expect_err("deux fois le même nom")
        .to_string();
    assert!(said.contains(&name), "le refus nomme l'application : {said}");
    drop(first);
}

#[test]
fn dropping_the_token_lets_the_next_one_in() {
    if !instance::supported() {
        return;
    }
    let name = format!("{}-again", a_name());
    {
        let _held = Only::one(&name).expect("libre");
    }
    // Rendu : sans cela, un logiciel qui s'arrête mal resterait « déjà
    // lancé » jusqu'au redémarrage de la machine.
    assert!(Only::one(&name).is_ok(), "le nom n'a pas été rendu");
}

#[test]
fn what_a_second_instance_hands_over_arrives() {
    if !instance::supported() {
        return;
    }
    let name = format!("{}-handover", a_name());
    let first = Only::one(&name).expect("libre");
    assert!(
        instance::handed_over().is_empty(),
        "personne n'a encore rien passé"
    );

    // Ce que fait la deuxième instance avant de s'en aller : passer ses
    // arguments à celle qui tourne. Fait ici depuis le même processus,
    // ce qui est le même chemin de bout en bout — la fenêtre trouvée
    // par son nom, le message, la file.
    instance::hand_over(&name, &["C:\\Travail\\été.txt".to_string()])
        .expect("la première instance est là");

    // Remis au fil de la fenêtre, donc pas forcément dans la
    // microseconde : attendu, mais pas longtemps.
    let mut handed = Vec::new();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while handed.is_empty() && std::time::Instant::now() < until {
        handed = instance::handed_over();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert_eq!(handed, vec!["C:\\Travail\\été.txt".to_string()]);
    assert!(
        instance::handed_over().is_empty(),
        "lu une fois : deux lectures ouvriraient deux fois le fichier"
    );
    drop(first);
}

#[test]
fn handing_over_to_nobody_says_so() {
    if !instance::supported() {
        return;
    }
    let refused = instance::hand_over("rustx-test-personne-ne-tourne", &[]);
    let said = refused
        .expect_err("personne ne tient ce nom")
        .to_string();
    assert!(
        said.contains("rustx-test-personne-ne-tourne"),
        "got {said}"
    );
}

#[test]
fn a_platform_without_this_says_so() {
    assert_eq!(instance::supported(), cfg!(windows));
}
