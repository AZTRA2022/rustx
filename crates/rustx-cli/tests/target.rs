//! Where cargo builds, and where this command line looks.
//!
//! The two have to be the same place, and they were not. `rustx build`
//! looked for the binary in `target/` beside the manifest and in
//! `target/` of every directory above it; cargo had been told to build
//! somewhere else entirely — by `CARGO_TARGET_DIR`, which is how a
//! developer shares one build directory between projects and how every
//! CI cache in the world is wired, or by `[build] target-dir` in a
//! `.cargo/config.toml` above the project, which `held` already read and
//! `produced` did not.
//!
//! What a developer saw was a build that worked followed by
//! `cargo said it was done and there is no mon-app.exe under any
//! target/` — the command line calling cargo a liar about a file cargo
//! had just written.
//!
//! The environment is handed in rather than read, so that these tests
//! say what it says without changing the environment of the process
//! they run in: tests share one process, and a variable set in one is a
//! variable set in all of them.

use std::fs;
use std::path::{Path, PathBuf};

use rustx_cli::build::{produced_told, target_directory, target_directory_told};

fn somewhere(what: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rustx-target-{what}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&path).expect("a directory to write into");
    path
}

/// A binary where cargo would have put it, so that looking for it is a
/// question with an answer.
fn a_binary(target: &Path, profile: &str, binary: &str) -> PathBuf {
    let out = target.join(profile);
    fs::create_dir_all(&out).expect("a profile directory");
    let exe = out.join(format!("{binary}{}", std::env::consts::EXE_SUFFIX));
    fs::write(&exe, b"what cargo wrote").expect("a built binary");
    exe
}

#[test]
fn the_directory_the_environment_names_is_the_one_cargo_builds_in() {
    let room = somewhere("environment");
    let project = room.join("mon-app");
    fs::create_dir_all(&project).expect("a project");
    let elsewhere = room.join("shared");

    assert_eq!(
        target_directory_told(&project, Some(&elsewhere.display().to_string())),
        elsewhere,
        "CARGO_TARGET_DIR is how cargo is told where to build, and it is not \
         a thing this command line may ignore"
    );
    let _ = fs::remove_dir_all(&room);
}

#[test]
fn a_relative_one_is_relative_to_the_project_cargo_is_run_in() {
    // `rustx build` runs cargo with the project as its working
    // directory, and cargo resolves a relative `CARGO_TARGET_DIR`
    // against the directory it was run in. So must this.
    let room = somewhere("relative");
    let project = room.join("mon-app");
    fs::create_dir_all(&project).expect("a project");

    assert_eq!(
        target_directory_told(&project, Some("build")),
        project.join("build")
    );
    let _ = fs::remove_dir_all(&room);
}

#[test]
fn an_empty_variable_is_not_a_directory() {
    // An exported-but-empty `CARGO_TARGET_DIR` is what a shell script
    // leaves behind, and cargo ignores it. Taking it at its word would
    // put the binary at the root of the filesystem.
    let room = somewhere("empty-variable");
    let project = room.join("mon-app");
    fs::create_dir_all(&project).expect("a project");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"mon-app\"\n",
    )
    .expect("a manifest");

    assert_eq!(
        target_directory_told(&project, Some("   ")),
        project.join("target")
    );
    let _ = fs::remove_dir_all(&room);
}

#[test]
fn a_member_of_a_workspace_builds_in_the_workspaces_own_target() {
    // Cargo builds every member of a workspace into one directory at
    // the workspace's root. A member looked at where it would have
    // built on its own is a member whose binary is never found.
    let room = somewhere("member");
    let project = room.join("mon-app");
    fs::create_dir_all(&project).expect("a project");
    fs::write(
        room.join("Cargo.toml"),
        "[workspace]\nmembers = [\"mon-app\"]\n",
    )
    .expect("a workspace");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"mon-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("a member");

    assert_eq!(target_directory_told(&project, None), room.join("target"));
    let _ = fs::remove_dir_all(&room);
}

#[test]
fn a_project_on_its_own_builds_beside_its_own_manifest() {
    let room = somewhere("alone");
    fs::write(
        room.join("Cargo.toml"),
        "[package]\nname = \"mon-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("a manifest");

    assert_eq!(target_directory_told(&room, None), room.join("target"));
    let _ = fs::remove_dir_all(&room);
}

#[test]
fn a_configuration_above_the_project_is_the_one_cargo_reads() {
    // Cargo walks up from where it was run looking for
    // `.cargo/config.toml`, and merges every one it finds. A project
    // inside a directory that configures a shared build directory
    // builds there, and only the walk finds that out.
    let room = somewhere("above");
    let project = room.join("mon-app");
    fs::create_dir_all(&project).expect("a project");
    fs::create_dir_all(room.join(".cargo")).expect("a .cargo directory");
    fs::write(
        room.join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"shared\"\n",
    )
    .expect("a configuration");

    assert_eq!(
        target_directory_told(&project, None),
        room.join("shared"),
        "a relative target-dir is relative to the directory that holds the \
         `.cargo` it was written in"
    );
    let _ = fs::remove_dir_all(&room);
}

#[test]
fn the_environment_beats_the_configuration_as_it_does_for_cargo() {
    let room = somewhere("both");
    fs::create_dir_all(room.join(".cargo")).expect("a .cargo directory");
    fs::write(
        room.join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"configured\"\n",
    )
    .expect("a configuration");
    let named = room.join("named");

    assert_eq!(
        target_directory_told(&room, Some(&named.display().to_string())),
        named
    );
    let _ = fs::remove_dir_all(&room);
}

#[test]
fn the_binary_is_looked_for_where_the_project_really_builds() {
    // The whole defect, in one assertion: cargo was told to build
    // somewhere else, it did, and the command line said there was no
    // binary anywhere.
    let room = somewhere("produced");
    let project = room.join("mon-app");
    fs::create_dir_all(&project).expect("a project");
    let elsewhere = room.join("shared");
    let exe = a_binary(&elsewhere, "debug", "mon-app");

    assert_eq!(
        produced_told(
            "mon-app",
            false,
            &project,
            Some(&elsewhere.display().to_string())
        ),
        Some(exe),
        "the binary cargo wrote is the binary `rustx build` hands back"
    );
    let _ = fs::remove_dir_all(&room);
}

#[test]
fn the_release_binary_and_the_debug_one_are_not_confused() {
    let room = somewhere("profiles");
    fs::write(
        room.join("Cargo.toml"),
        "[package]\nname = \"mon-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("a manifest");
    let debug = a_binary(&room.join("target"), "debug", "mon-app");

    assert_eq!(produced_told("mon-app", false, &room, None), Some(debug));
    assert_eq!(
        produced_told("mon-app", true, &room, None),
        None,
        "a release build that was never made must not hand back the debug one"
    );

    let release = a_binary(&room.join("target"), "release", "mon-app");
    assert_eq!(produced_told("mon-app", true, &room, None), Some(release));
    let _ = fs::remove_dir_all(&room);
}

#[test]
fn a_binary_of_a_member_is_found_in_the_workspaces_target() {
    let room = somewhere("member-binary");
    let project = room.join("mon-app");
    fs::create_dir_all(&project).expect("a project");
    fs::write(
        room.join("Cargo.toml"),
        "[workspace]\nmembers = [\"mon-app\"]\n",
    )
    .expect("a workspace");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"mon-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("a member");
    let exe = a_binary(&room.join("target"), "debug", "mon-app");

    assert_eq!(produced_told("mon-app", false, &project, None), Some(exe));
    let _ = fs::remove_dir_all(&room);
}

/// Whatever the environment of this test run happens to say, the
/// function that reads it and the function that looks for the binary
/// must agree — a disagreement between the two is the defect itself.
#[test]
fn what_is_watched_for_being_held_is_what_is_looked_in_for_the_binary() {
    let room = somewhere("agreement");
    fs::write(
        room.join("Cargo.toml"),
        "[package]\nname = \"mon-app\"\nversion = \"0.1.0\"\n",
    )
    .expect("a manifest");

    let target = target_directory(&room);
    assert_eq!(
        target,
        target_directory_told(
            &room,
            rustx_cli::build::target_from_environment().as_deref()
        ),
        "the only difference between the two is who reads the environment"
    );
    let _ = fs::remove_dir_all(&room);
}
