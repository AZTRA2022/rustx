//! An application installing itself into a redirected profile, on whichever
//! system this runs on, and removing itself again.

use rustx_os::install::{self, App};

#[test]
fn an_app_installs_itself_registers_and_is_removed() {
    let root = std::env::temp_dir().join(format!("rustx-install-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (local, roam, home) = (root.join("local"), root.join("roam"), root.join("home"));
    for dir in [&local, &roam, &home] {
        std::fs::create_dir_all(dir).unwrap();
    }
    // SAFETY: the only test of this file, so nothing else reads the environment.
    unsafe {
        std::env::set_var("LOCALAPPDATA", &local);
        std::env::set_var("APPDATA", &roam);
        std::env::set_var("HOME", &home);
    }

    // A binary to copy: inside an .app on macOS, which is what gets installed there.
    let binary = if cfg!(target_os = "macos") {
        root.join("Download/Fake.app/Contents/MacOS/fake")
    } else {
        root.join("Download/fake")
    };
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    std::fs::write(&binary, vec![42_u8; 10_000]).unwrap();
    let app = App {
        name: "Rustx Install Test".into(),
        version: "1.2.3".into(),
        publisher: "rustx".into(),
        binary: binary.clone(),
    };
    assert!(!install::is_installed_here(&app));
    assert_eq!(install::installed_version(&app.name), None);

    let mut seen = Vec::new();
    let installed = install::install(&app, &mut |percent| seen.push(percent)).expect("installs");
    assert_eq!(seen.last(), Some(&100));
    assert_eq!(std::fs::read(&installed).unwrap().len(), 10_000);
    assert_eq!(
        install::installed_version(&app.name).as_deref(),
        Some("1.2.3")
    );
    let there = App {
        binary: installed.clone(),
        ..app.clone()
    };
    assert!(install::is_installed_here(&there));

    install::uninstall(&app.name).expect("removes");
    // Windows removes the folder after this process ends; elsewhere it is gone now.
    if !cfg!(windows) {
        assert!(!install::destination(&app.name).unwrap().exists());
        assert_eq!(install::installed_version(&app.name), None);
    }
    let _ = std::fs::remove_dir_all(&root);
}
