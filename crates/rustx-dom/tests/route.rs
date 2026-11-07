//! Matching a path against the table the compiler generated.

use rustx_dom::Element;
use rustx_dom::route::{Params, Route, match_route};

fn nothing(_params: &Params) -> Element {
    Element::Empty
}

/// A table of static patterns, the way the generated one is shaped.
fn table(patterns: &[&'static str]) -> Vec<Route> {
    patterns
        .iter()
        .map(|pattern| Route {
            pattern,
            render: nothing,
            layouts: &[],
            loading: None,
        })
        .collect()
}

/// Which pattern `path` matched, if any.
fn matched(path: &str, patterns: &[&'static str]) -> Option<&'static str> {
    let table = table(patterns);
    match_route(&table, path).map(|(route, _)| route.pattern)
}

#[test]
fn a_static_path_matches_its_own_route() {
    assert_eq!(matched("/device", &["/", "/device"]), Some("/device"));
}

#[test]
fn the_root_matches_the_root() {
    assert_eq!(matched("/", &["/", "/device"]), Some("/"));
}

#[test]
fn a_bracketed_segment_captures_what_is_there() {
    let table = table(&["/device/[id]"]);
    let (_, params) = match_route(&table, "/device/7").expect("a match");
    assert_eq!(params.get("id"), "7");
}

#[test]
fn a_static_segment_wins_over_a_parameter() {
    // Review Focus 4, the second half. Whatever order the table is in.
    assert_eq!(
        matched("/device/new", &["/device/[id]", "/device/new"]),
        Some("/device/new")
    );
    assert_eq!(
        matched("/device/new", &["/device/new", "/device/[id]"]),
        Some("/device/new")
    );
}

#[test]
fn a_path_nobody_declared_matches_nothing() {
    // Review Focus 4, the first half: the router turns this into the
    // not-found page rather than a blank window.
    assert_eq!(matched("/nowhere", &["/", "/device"]), None);
}

#[test]
fn a_trailing_slash_is_the_same_path() {
    assert_eq!(matched("/device/", &["/device"]), Some("/device"));
}

#[test]
fn a_parameter_does_not_match_across_a_slash() {
    assert_eq!(matched("/device/7/edit", &["/device/[id]"]), None);
}

#[test]
fn a_shorter_path_does_not_match_a_longer_pattern() {
    assert_eq!(matched("/device", &["/device/[id]"]), None);
}

#[test]
fn an_empty_segment_does_not_fill_a_parameter() {
    // `/device//` would otherwise hand a page an empty id.
    assert_eq!(matched("/device//", &["/device/[id]"]), None);
}

#[test]
fn two_parameters_are_both_captured() {
    let table = table(&["/device/[id]/stream/[format]"]);
    let (_, params) = match_route(&table, "/device/7/stream/mjpeg").expect("a match");
    assert_eq!(params.get("id"), "7");
    assert_eq!(params.get("format"), "mjpeg");
}

#[test]
fn a_parameter_nobody_captured_reads_as_nothing() {
    // A page asking for a name the path never had gets "", not a panic:
    // this is generated code's caller, and it must not take the window
    // down over a mistake the developer can see on screen.
    let table = table(&["/device/[id]"]);
    let (_, params) = match_route(&table, "/device/7").expect("a match");
    assert_eq!(params.get("nope"), "");
}

#[test]
fn a_path_with_no_leading_slash_is_still_a_path() {
    assert_eq!(matched("device", &["/device"]), Some("/device"));
}

// ──────────────────────────── La query string ────────────────────────────

#[test]
fn a_query_string_does_not_stop_a_path_from_matching() {
    // `/depots?q=rust` ne correspondait à rien : le `?` faisait partie
    // du dernier segment. Une recherche partageable était donc
    // impossible, et le labo garde son terme dans un état de page
    // faute de pouvoir le mettre dans l'URL.
    assert_eq!(matched("/depots?q=rust", &["/", "/depots"]), Some("/depots"));
}

#[test]
fn what_the_query_string_says_is_read_by_name() {
    let table = table(&["/depots"]);
    let (_, params) = match_route(&table, "/depots?q=rust&page=2").expect("a match");
    assert_eq!(params.query("q"), "rust");
    assert_eq!(params.query("page"), "2");
    // Une clé que personne n'a écrite est vide, comme pour un segment.
    assert_eq!(params.query("tri"), "");
}

#[test]
fn a_query_value_is_decoded_as_a_browser_would() {
    let table = table(&["/depots"]);
    let (_, params) =
        match_route(&table, "/depots?q=rust%20lang&ou=a%2Fb&et=un+deux").expect("a match");
    assert_eq!(params.query("q"), "rust lang");
    assert_eq!(params.query("ou"), "a/b");
    // Dans une query string, `+` est une espace : c'est ce que tout
    // navigateur écrit et ce que tout serveur relit.
    assert_eq!(params.query("et"), "un deux");
}

#[test]
fn a_key_without_a_value_is_there_and_empty() {
    let table = table(&["/depots"]);
    let (_, params) = match_route(&table, "/depots?brut&q=").expect("a match");
    assert!(params.has("brut"), "la clé est là");
    assert_eq!(params.query("brut"), "");
    assert!(params.has("q"));
    assert!(!params.has("rien"));
}

#[test]
fn a_fragment_is_not_part_of_the_path_either() {
    assert_eq!(matched("/depots#ici", &["/depots"]), Some("/depots"));
}

#[test]
fn a_segment_and_a_query_key_of_the_same_name_do_not_mix() {
    let table = table(&["/device/[id]"]);
    let (_, params) = match_route(&table, "/device/7?id=9").expect("a match");
    assert_eq!(params.get("id"), "7", "le segment est le segment");
    assert_eq!(params.query("id"), "9", "la query est la query");
}

// ───────────────────────── Le segment attrape-tout ─────────────────────────

#[test]
fn a_catch_all_takes_the_rest_of_the_path() {
    let table = table(&["/fichiers/[...chemin]"]);
    let (_, params) = match_route(&table, "/fichiers/a/b/c.txt").expect("a match");
    assert_eq!(params.get("chemin"), "a/b/c.txt");
}

#[test]
fn a_catch_all_takes_one_segment_too() {
    let table = table(&["/fichiers/[...chemin]"]);
    let (_, params) = match_route(&table, "/fichiers/seul").expect("a match");
    assert_eq!(params.get("chemin"), "seul");
}

#[test]
fn a_catch_all_does_not_take_nothing() {
    // `/fichiers` n'est pas `/fichiers/[...chemin]` : c'est la règle de
    // Next.js, et c'est ce qui laisse un écran « racine » exister.
    assert_eq!(matched("/fichiers", &["/fichiers/[...chemin]"]), None);
}

#[test]
fn an_optional_catch_all_takes_nothing_as_well() {
    let table = table(&["/fichiers/[[...chemin]]"]);
    let (_, params) = match_route(&table, "/fichiers").expect("a match");
    assert_eq!(params.get("chemin"), "");
    let (_, params) = match_route(&table, "/fichiers/a/b").expect("a match");
    assert_eq!(params.get("chemin"), "a/b");
}

#[test]
fn a_named_segment_beats_a_catch_all() {
    assert_eq!(
        matched("/fichiers/7", &["/fichiers/[...chemin]", "/fichiers/[id]"]),
        Some("/fichiers/[id]")
    );
}

#[test]
fn a_static_segment_beats_a_catch_all() {
    assert_eq!(
        matched("/fichiers/neuf", &["/fichiers/[...chemin]", "/fichiers/neuf"]),
        Some("/fichiers/neuf")
    );
}

#[test]
fn the_longest_known_prefix_beats_the_shortest() {
    assert_eq!(
        matched(
            "/fichiers/images/chat.png",
            &["/fichiers/[...chemin]", "/fichiers/images/[...chemin]"]
        ),
        Some("/fichiers/images/[...chemin]")
    );
}

#[test]
fn a_catch_all_may_be_the_whole_route() {
    let table = table(&["/[...tout]"]);
    let (_, params) = match_route(&table, "/a/b").expect("a match");
    assert_eq!(params.get("tout"), "a/b");
}

// ───────────────────── Écrire une query string ─────────────────────

#[test]
fn a_key_is_set_where_it_already_was() {
    use rustx_dom::route::with_query;
    // L'ordre est gardé : une URL qui se rebat les cartes à chaque
    // filtre est une URL que personne ne peut lire.
    assert_eq!(
        with_query("/depots?q=rust&tri=etoiles", "q", Some("tokio")),
        "/depots?q=tokio&tri=etoiles"
    );
}

#[test]
fn a_key_that_was_not_there_is_added_at_the_end() {
    use rustx_dom::route::with_query;
    assert_eq!(
        with_query("/depots?q=rust", "page", Some("2")),
        "/depots?q=rust&page=2"
    );
    assert_eq!(with_query("/depots", "q", Some("rust")), "/depots?q=rust");
}

#[test]
fn nothing_removes_the_key_and_the_last_one_removes_the_question_mark() {
    use rustx_dom::route::with_query;
    assert_eq!(
        with_query("/depots?q=rust&page=2", "page", None),
        "/depots?q=rust"
    );
    assert_eq!(with_query("/depots?q=rust", "q", None), "/depots");
}

#[test]
fn a_value_with_a_space_or_a_slash_is_escaped_and_read_back() {
    use rustx_dom::route::{query_of, split_query, with_query};
    let written = with_query("/depots", "q", Some("rust lang/core"));
    assert!(
        !written.contains(' '),
        "une espace dans une URL n'est pas une URL : {written}"
    );
    let (_, query) = split_query(&written);
    assert_eq!(query_of(query, "q"), "rust lang/core");
}
