//! `<js::recharts::LineChart />` — the tag that is a component off npm.

use rustx_compiler::transpile::transpile;

fn rewrite_one(source: &str) -> String {
    transpile(source).expect("transpilable").rust
}

fn refusal(source: &str) -> String {
    transpile(source).expect_err("should be refused").message
}

#[test]
fn a_js_tag_becomes_a_foreign_element() {
    let rust =
        rewrite_one("fn P() -> Element {\n    return <js::Chart series={\"a\".to_string()} />;\n}");
    assert!(rust.contains("Element :: Foreign"), "got {rust}");
    assert!(rust.contains("\"Chart\""), "got {rust}");
    assert!(rust.contains("\"series\""), "got {rust}");
}

#[test]
fn a_library_component_is_named_by_its_module() {
    // What the developer writes is the import they would have written in
    // JavaScript: the module, then the export.
    let rust =
        rewrite_one("fn P() -> Element {\n    return <js::recharts::LineChart width={320} />;\n}");
    assert!(rust.contains("\"recharts::LineChart\""), "got {rust}");
}

#[test]
fn a_js_tag_gets_an_id_of_its_own() {
    let rust =
        rewrite_one("fn P() -> Element {\n    return <div><js::Chart /><js::Chart /></div>;\n}");
    let ids: Vec<&str> = rust.matches("rustx-js-").collect();
    assert_eq!(
        ids.len(),
        2,
        "each boundary needs its own container: {rust}"
    );
    // Deterministic, like a `Slot`: the line and the column it was
    // written at, so the engine finds the container without searching.
    assert!(rust.contains("rustx-js-2-17"), "got {rust}");
    assert!(rust.contains("rustx-js-2-30"), "got {rust}");
}

#[test]
fn a_prop_crosses_through_to_value() {
    // The compiler does not know the types of the developer's Rust, and
    // it does not have to: `ToValue` is what says a type can cross, and
    // a type that cannot is refused by `rustc` at the line in the `.rsx`
    // that wrote it — which is more than a guess in the compiler could
    // ever say.
    let rust = rewrite_one(
        "fn P(points: Vec<Point>) -> Element {\n    \
         return <js::recharts::LineChart data={points.clone()} />;\n}",
    );
    assert!(rust.contains("ToValue :: to_value"), "got {rust}");
    assert!(rust.contains("points . clone ()"), "got {rust}");
}

#[test]
fn a_foreign_tag_may_hold_foreign_children() {
    // A chart is written with its axes inside it, the way it is written
    // in JSX — and the whole tree is handed over at once.
    let rust = rewrite_one(
        "fn P() -> Element {\n    return <js::recharts::LineChart width={320}>\n        \
         <js::recharts::XAxis dataKey=\"at\" />\n    </js::recharts::LineChart>;\n}",
    );
    assert!(rust.contains("\"recharts::XAxis\""), "got {rust}");
    assert!(rust.contains("ForeignChild :: Node"), "got {rust}");
}

#[test]
fn text_inside_a_foreign_tag_is_a_child_of_it() {
    let rust = rewrite_one("fn P() -> Element {\n    return <js::Button>Save</js::Button>;\n}");
    assert!(rust.contains("ForeignChild :: Text"), "got {rust}");
}

#[test]
fn a_lowercase_js_tag_is_still_refused_as_an_unknown_tag() {
    assert!(transpile("fn P() -> Element {\n    return <js::chart />;\n}").is_err());
}

#[test]
fn a_lowercase_export_in_a_module_is_refused_by_name() {
    let message = refusal("fn P() -> Element {\n    return <js::recharts::lineChart />;\n}");
    assert!(message.contains("LineChart"), "got {message}");
}

#[test]
fn a_number_and_a_flag_cross_as_what_they_are() {
    let rust = rewrite_one("fn P() -> Element {\n    return <js::Chart height={240} live />;\n}");
    assert!(rust.contains("ToValue :: to_value"), "got {rust}");
    assert!(rust.contains("Bool (true)"), "got {rust}");
}

#[test]
fn a_literal_prop_crosses_as_a_string() {
    let rust = rewrite_one("fn P() -> Element {\n    return <js::Chart title=\"Live\" />;\n}");
    assert!(rust.contains("String (\"Live\""), "got {rust}");
}

#[test]
fn a_prop_the_library_calls_is_a_closure_that_stays_in_rust() {
    // Every real library has one: `onValueChange`, `onSelect`,
    // `onRowClick`. The closure is not sent anywhere — what crosses is
    // a function that calls it.
    let rust = rewrite_one(
        "fn P() -> Element {
    return <Tabs.Root onValueChange={move |value| drop(value)} />;
}",
    );
    assert!(rust.contains("Callback :: new"), "got {rust}");
    assert!(rust.contains("callbacks"), "got {rust}");
}

#[test]
fn a_prop_the_library_calls_with_no_closure_is_refused_by_name() {
    let message = refusal(
        "fn P() -> Element {
    return <Tabs.Root onValueChange=\"two\" />;
}",
    );
    assert!(message.contains("onValueChange"), "got {message}");
    assert!(message.contains("closure"), "got {message}");
}

#[test]
fn a_capitalised_tag_the_file_does_not_declare_is_a_library_component() {
    // No prefix. A tag the page declares is the page's; a tag it does
    // not is a component off npm, found by its name among the libraries
    // the application installed.
    let rust = rewrite_one("fn P() -> Element {\n    return <LineChart width={320} />;\n}");
    assert!(rust.contains("Element :: Foreign"), "got {rust}");
    assert!(rust.contains("\"LineChart\""), "got {rust}");
}

#[test]
fn a_dotted_tag_is_a_path_into_a_library() {
    // `<Dialog.Trigger />`, as it is written in JSX.
    let rust =
        rewrite_one("fn P() -> Element {\n    return <Dialog.Trigger>open</Dialog.Trigger>;\n}");
    assert!(rust.contains("\"Dialog.Trigger\""), "got {rust}");
    assert!(rust.contains("ForeignChild :: Text"), "got {rust}");
}

#[test]
fn a_tag_the_file_declares_is_still_a_rustx_component() {
    let rust = rewrite_one(
        "fn P() -> Element {\n    return <Row name={\"a\".to_string()} />;\n}\n\
         fn Row(name: String) -> Element {\n    return <li>{name}</li>;\n}",
    );
    assert!(!rust.contains("Element :: Foreign"), "got {rust}");
    assert!(rust.contains("component_at"), "got {rust}");
}

#[test]
fn a_plain_tag_inside_a_library_tag_is_handed_over_as_itself() {
    // React builds what is inside the boundary, so a `<div>` there is
    // React's `createElement("div", …)` — not a node rustx thinks it
    // owns. It is how anyone wraps the inside of a dialog.
    let rust = rewrite_one(
        "fn P() -> Element {\n    return <Content><div className=\"flex\">hi</div></Content>;\n}",
    );
    assert!(rust.contains("\"Content\""), "got {rust}");
    assert!(rust.contains("\"div\""), "got {rust}");
    assert!(rust.contains("ForeignChild :: Node"), "got {rust}");
}

#[test]
fn an_unknown_tag_inside_a_library_tag_is_still_refused() {
    let message = refusal("fn P() -> Element {\n    return <Content><dvi>hi</dvi></Content>;\n}");
    assert!(message.contains("dvi"), "got {message}");
}

#[test]
fn a_library_named_before_the_component_needs_no_prefix() {
    // The explicit form, for a name two libraries export — and two
    // libraries export plenty: `lucide-react` has an icon called
    // `LineChart`, and so has recharts a chart. A page that has to
    // disambiguate must not have to reach for `js::` to do it, since
    // that prefix is exactly what this framework took out.
    let rust =
        rewrite_one("fn P() -> Element {\n    return <recharts::LineChart width={320} />;\n}");
    assert!(rust.contains("\"recharts::LineChart\""), "got {rust}");
    assert!(
        !rust.contains("tag(\"recharts::LineChart\")"),
        "it was taken for an HTML tag: {rust}"
    );
}

#[test]
fn a_component_of_the_applications_own_library_is_named_the_same_way() {
    // `components/ui`, which is where `shadcn add` leaves its work, is a
    // library like any other and is called `ui`.
    let rust = rewrite_one("fn P() -> Element {\n    return <ui::Button id=\"go\" />;\n}");
    assert!(rust.contains("\"ui::Button\""), "got {rust}");
}

#[test]
fn a_callback_of_two_arguments_is_given_two() {
    // Une bibliothèque qui appelle `onChange(valeur, index)` existe —
    // les listes virtuelles, les tableaux, les sélecteurs de plage en
    // ont tous. Le compilateur n'appelait la closure qu'avec le premier
    // argument, donc une closure à deux paramètres ne compilait pas.
    let rust = rewrite_one(
        "fn Page() -> Element {\n    \
         return <Liste.Root onChange={move |valeur, index| drop((valeur, index))} />;\n}\n",
    );
    assert!(
        rust.contains("nth (__arguments , 0usize)") || rust.contains("nth(__arguments, 0usize)"),
        "le premier argument doit être pris par son rang : {rust}"
    );
    assert!(
        rust.contains("nth (__arguments , 1usize)") || rust.contains("nth(__arguments, 1usize)"),
        "et le second aussi : {rust}"
    );
}

#[test]
fn a_callback_of_no_argument_is_given_none() {
    // `onOpenChange={move || ferme()}` : un bouton de fermeture ne
    // reçoit rien, et une closure sans paramètre à qui on passe un
    // argument ne compile pas.
    let rust = rewrite_one(
        "fn Page() -> Element {\n    \
         return <Dialog.Root onClose={move || drop(1)} />;\n}\n",
    );
    assert!(
        rust.contains("__called ()") || rust.contains("__called()"),
        "rien ne doit être passé : {rust}"
    );
}

#[test]
fn a_callback_of_one_argument_still_gets_the_first() {
    let rust = rewrite_one(
        "fn Page() -> Element {\n    \
         return <Tabs.Root onValueChange={move |value| drop(value)} />;\n}\n",
    );
    assert!(
        rust.contains("nth (__arguments , 0usize)") || rust.contains("nth(__arguments, 0usize)"),
        "{rust}"
    );
}

// ───── Une balise ordinaire dans un sous-arbre React ─────

#[test]
fn an_event_on_a_plain_tag_inside_react_is_still_an_event() {
    // Ce que le labo a rencontré : `<input onInput={move |event| …}>`
    // écrit dans une carte shadcn. React monte la balise, donc le
    // gestionnaire revient par le chemin des props — et la closure
    // recevait une valeur, quand la même balise montée par le moteur
    // reçoit un évènement. Le `.rsx` était le même ; seul un ancêtre
    // changeait.
    let emitted = rewrite_one(
        "fn Page() -> Element {\n    \
         return <Card><input onInput={move |event| { let _ = event.value; }} /></Card>;\n}",
    );
    assert!(
        emitted.contains("Event :: from_value") || emitted.contains("Event::from_value"),
        "l'évènement ne traverse pas : {emitted}"
    );
}

#[test]
fn a_library_prop_that_is_not_a_dom_event_stays_a_value() {
    // `onValueChange` n'est pas un évènement du DOM : ce qu'une
    // bibliothèque y passe est ce qu'elle veut, et le lire comme un
    // évènement perdrait la chaîne qu'elle a envoyée.
    let emitted = rewrite_one(
        "fn Page() -> Element {\n    \
         return <Select onValueChange={move |valeur| { let _ = valeur; }} />;\n}",
    );
    assert!(
        !emitted.contains("Event :: from_value") && !emitted.contains("Event::from_value"),
        "une prop de bibliothèque a été lue comme un évènement : {emitted}"
    );
}
