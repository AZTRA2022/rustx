//! Le gestionnaire de fichiers : ce qui se prouve sans ouvrir de
//! fenêtre. Une association s'écrit et se relit dans le registre, puis
//! se défait ; montrer un fichier ouvrirait l'Explorateur sous les yeux
//! de celui qui lance les tests, et n'est donc vérifié que dans son
//! refus d'un fichier qui n'existe pas.

#![cfg(windows)]

use std::path::Path;

use rustx_os::shell::{self, Association};

#[test]
fn an_association_is_written_then_undone() {
    let association = Association {
        extension: format!("rustxtest{}", std::process::id()),
        id: format!("Rustx.Test{}", std::process::id()),
        description: "rustx test document".to_string(),
        program: r"C:\Program Files\Rustx Test\app.exe".into(),
    };
    shell::associate(&association).expect("written");

    let read = |key: &str| {
        let output = std::process::Command::new("reg")
            .args(["query", &format!(r"HKCU\Software\Classes\{key}"), "/ve"])
            .output()
            .expect("reg.exe");
        String::from_utf8_lossy(&output.stdout).to_string()
    };
    assert!(read(&format!(".{}", association.extension)).contains(&association.id));
    let command = read(&format!(r"{}\shell\open\command", association.id));
    assert!(
        command.contains(r#""C:\Program Files\Rustx Test\app.exe" "%1""#),
        "{command}"
    );

    shell::dissociate(&association).expect("undone");
    assert!(!read(&format!(".{}", association.extension)).contains(&association.id));
    assert!(!read(&association.id).contains("rustx test document"));
    // Twice is once.
    shell::dissociate(&association).expect("undone again");
}

#[test]
fn nothing_is_revealed_or_remembered_that_does_not_exist() {
    assert!(shell::reveal(Path::new("no/such/file.rx")).is_err());
    assert!(shell::remember_recent(Path::new("no/such/file.rx")).is_err());
}
