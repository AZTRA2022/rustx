//! What an application's JavaScript brings to the editor's list.
//!
//! Written against a real directory rather than against strings: what
//! broke before was never the parsing, it was where the files are —
//! which field of a package's manifest points at its declarations, and
//! whether `components/ui` is beside the manifest or beside the pages.

use std::path::{Path, PathBuf};

use rustx_lsp::libraries;

/// An application on disk, thrown away with the test.
struct Application(PathBuf);

impl Drop for Application {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn written(at: &Path, what: &str) {
    if let Some(parent) = at.parent() {
        std::fs::create_dir_all(parent).expect("a directory");
    }
    std::fs::write(at, what).expect("a file");
}

/// An application with one package installed and one component of its
/// own, the way `npm install` and `npx shadcn add` leave them.
fn an_application(name: &str) -> Application {
    let root = std::env::temp_dir().join(format!("rustx-libraries-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a directory");

    written(
        &root.join("package.json"),
        "{\n  \"dependencies\": {\n    \"recharts\": \"2.12.7\",\n    \
         \"react\": \"18.3.1\",\n    \"clsx\": \"^2.1.1\"\n  }\n}",
    );
    // recharts points at its declarations with a `types` field.
    written(
        &root.join("node_modules/recharts/package.json"),
        "{\n  \"name\": \"recharts\",\n  \"types\": \"./types/index.d.ts\"\n}",
    );
    written(
        &root.join("node_modules/recharts/types/index.d.ts"),
        "export { LineChart } from './chart/LineChart';\n\
         export { XAxis } from './cartesian/XAxis';\n\
         export type { LineChartProps } from './chart/LineChart';\n",
    );
    // `clsx` is in the manifest and is not a component library.
    written(
        &root.join("node_modules/clsx/index.d.ts"),
        "export declare function clsx(...inputs: unknown[]): string;\nexport { clsx as Clsx };\n",
    );
    // And what `shadcn add card` writes.
    written(
        &root.join("components/ui/card.tsx"),
        "const Card = () => null;\nconst CardHeader = () => null;\n\
         export { Card, CardHeader };\nexport const cn = 1;\n",
    );
    Application(root)
}

#[test]
fn a_components_name_is_read_from_the_package_it_is_installed_from() {
    let application = an_application("recharts");
    let found = libraries::read(&application.0);
    let names: Vec<&str> = found.iter().map(|export| export.name.as_str()).collect();
    assert!(names.contains(&"LineChart"), "got {names:?}");
    assert!(names.contains(&"XAxis"), "got {names:?}");
    let chart = found
        .iter()
        .find(|export| export.name == "LineChart")
        .expect("LineChart");
    assert_eq!(chart.from, "recharts", "it must say where it comes from");
}

#[test]
fn the_applications_own_components_are_read_too() {
    let application = an_application("own");
    let found = libraries::read(&application.0);
    let own: Vec<&str> = found
        .iter()
        .filter(|export| export.from == "ui")
        .map(|export| export.name.as_str())
        .collect();
    // `CardHeader` is the one a list of file names could never give:
    // `card.tsx` exports four components, and three of them are not
    // called `Card`.
    assert_eq!(own, vec!["Card", "CardHeader"], "got {own:?}");
}

#[test]
fn what_is_not_a_component_is_not_offered_as_a_tag() {
    let application = an_application("helpers");
    let found = libraries::read(&application.0);
    let names: Vec<&str> = found.iter().map(|export| export.name.as_str()).collect();
    assert!(!names.contains(&"cn"), "a helper is not a tag: {names:?}");
    assert!(
        !names.contains(&"LineChartProps"),
        "a type is not a tag: {names:?}"
    );
    assert!(
        !names.iter().any(|name| *name == "Clsx"),
        "a library of functions is not a library of components: {names:?}"
    );
}

#[test]
fn an_answer_is_remembered_until_the_application_changes() {
    let application = an_application("cache");
    let first = libraries::exports_for(&application.0);
    assert!(!first.is_empty());
    // The same answer, from the cache: nothing on disk has moved.
    assert_eq!(libraries::exports_for(&application.0), first);

    // And a component added now is a tag now — which is the whole point
    // of a stamp rather than a once-and-for-all read.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    written(
        &application.0.join("components/ui/badge.tsx"),
        "export const Badge = () => null;\n",
    );
    let again = libraries::exports_for(&application.0);
    assert!(
        again.iter().any(|export| export.name == "Badge"),
        "the new component was not read: {:?}",
        again.iter().map(|e| &e.name).collect::<Vec<_>>()
    );
}
