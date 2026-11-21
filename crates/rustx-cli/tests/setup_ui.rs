//! The Windows setup, driven through its own pages: a project's custom
//! `install.html` and `uninstall.html` run in the system web view, call
//! `setup.install()` / `setup.uninstall()` themselves, and close when done.
//! The profile is redirected to a temp folder so nothing real is touched.
#![cfg(windows)]

use rustx_cli::selfextract::{Meta, pack};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn temp(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("rustx-setup-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn run(exe: &Path, args: &[&str], local: &Path, roam: &Path) -> bool {
    let mut child = Command::new(exe)
        .args(args)
        .env("LOCALAPPDATA", local)
        .env("APPDATA", roam)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the setup starts");
    let started = Instant::now();
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return status.success();
        }
        if started.elapsed() > Duration::from_secs(90) {
            let _ = child.kill();
            return false;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[test]
fn a_project_page_drives_the_install_and_the_uninstall() {
    let root = temp("pages");
    let folder = root.join("app");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("testapp.exe"), b"not really an exe").unwrap();
    // The project's own pages: they act at once and close at the end.
    std::fs::write(
        root.join("install.html"),
        "<p id=name></p><script>document.getElementById('name').textContent = setup.info.name;\
         setup.on(e => { if (e.event === 'done') setup.close(); }); setup.install();</script>",
    )
    .unwrap();
    std::fs::write(
        root.join("uninstall.html"),
        // The person reads "Removed" for a while before closing: the folder must still go.
        "<script>setup.on(e => { if (e.event === 'done') setTimeout(() => setup.close(), 5000); }); setup.uninstall();</script>",
    )
    .unwrap();
    let pages = vec![
        ("install.html".to_string(), root.join("install.html")),
        ("uninstall.html".to_string(), root.join("uninstall.html")),
    ];
    let meta = Meta {
        name: "Rustx Setup Test".into(),
        version: "1.0.0".into(),
        publisher: "rustx".into(),
        binary: "testapp.exe".into(),
        files: 0,
    };
    let setup = root.join("setup.exe");
    pack(
        Path::new(env!("CARGO_BIN_EXE_rustx-setup")),
        &folder,
        &pages,
        &meta,
        &setup,
    )
    .unwrap();

    let (local, roam) = (root.join("local"), root.join("roam"));
    std::fs::create_dir_all(&local).unwrap();
    std::fs::create_dir_all(&roam).unwrap();
    assert!(
        run(&setup, &[], &local, &roam),
        "the install page did not finish"
    );

    let installed = local.join("Programs").join("Rustx Setup Test");
    assert!(
        installed.join("testapp.exe").is_file(),
        "the application is not there"
    );
    assert!(installed.join("uninstall.exe").is_file());
    assert!(
        installed.join("installer").join("uninstall.html").is_file(),
        "the uninstall page is kept"
    );

    let uninstaller = installed.join("uninstall.exe");
    assert!(
        run(
            &uninstaller,
            &["--uninstall", &installed.display().to_string()],
            &local,
            &roam
        ),
        "the uninstall page did not finish"
    );
    let started = Instant::now();
    while installed.exists() && started.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(250));
    }
    assert!(!installed.exists(), "the folder is still there");
    let _ = std::fs::remove_dir_all(root);
}
