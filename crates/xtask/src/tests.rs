use super::*;

#[test]
fn module_path_for_lib_root_is_the_crate_name() {
    assert_eq!(
        module_path_for("nivela_core", Path::new("lib.rs")),
        "nivela_core"
    );
}

#[test]
fn module_path_for_a_plain_file_appends_its_stem() {
    assert_eq!(
        module_path_for("nivela_core", Path::new("probe.rs")),
        "nivela_core::probe"
    );
}

#[test]
fn module_path_for_a_mod_rs_drops_the_mod_segment() {
    assert_eq!(
        module_path_for("nivela_app", Path::new("screens/mod.rs")),
        "nivela_app::screens"
    );
}

#[test]
fn module_path_for_a_nested_file_joins_with_double_colons() {
    assert_eq!(
        module_path_for("nivela_app", Path::new("screens/home.rs")),
        "nivela_app::screens::home"
    );
}

#[test]
fn extracts_a_documented_public_function() {
    let src = "/// Adds two numbers.\npub fn add(a: i32, b: i32) -> i32 { a + b }\n";
    let items = extract_items(src);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, "fn");
    assert_eq!(items[0].name, "add");
    assert_eq!(items[0].doc.as_deref(), Some("Adds two numbers."));
}

#[test]
fn extracts_a_public_struct_without_a_doc_comment() {
    let src = "pub struct Point { x: f32, y: f32 }\n";
    let items = extract_items(src);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, "struct");
    assert_eq!(items[0].name, "Point");
    assert_eq!(items[0].doc, None);
}

#[test]
fn doc_comment_survives_a_derive_attribute_in_between() {
    let src = "/// A 2D point.\n#[derive(Debug, Clone)]\npub struct Point { x: f32, y: f32 }\n";
    let items = extract_items(src);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].doc.as_deref(), Some("A 2D point."));
}

#[test]
fn ignores_private_items() {
    let src = "fn helper() {}\nstruct Internal;\n";
    assert!(extract_items(src).is_empty());
}

#[test]
fn resolves_crate_relative_use_edges() {
    let edges = extract_edges(
        "use crate::media::MediaAsset;\n",
        "nivela_core",
        "nivela_core::probe",
    );
    assert!(edges.contains(&("uses", "nivela_core::media".to_string())));
}

#[test]
fn resolves_super_relative_use_edges() {
    // `widgets` is a sibling module (lowercase), so it's kept as the target rather than
    // treated as an item to drop.
    let edges = extract_edges(
        "use super::widgets;\n",
        "nivela_app",
        "nivela_app::screens::home",
    );
    assert!(edges.contains(&("uses", "nivela_app::screens::widgets".to_string())));
}

#[test]
fn resolves_super_relative_use_edges_for_an_item() {
    let edges = extract_edges(
        "use super::NivelaApp;\n",
        "nivela_app",
        "nivela_app::screens::home",
    );
    assert!(edges.contains(&("uses", "nivela_app::screens".to_string())));
}

#[test]
fn tracks_declared_submodules() {
    let edges = extract_edges("pub mod probe;\n", "nivela_core", "nivela_core");
    assert!(edges.contains(&("declares", "nivela_core::probe".to_string())));
}

#[test]
fn resolve_import_target_drops_a_trailing_item_but_keeps_a_trailing_module() {
    assert_eq!(
        resolve_import_target("nivela_core::media::MediaAsset"),
        "nivela_core::media"
    );
    assert_eq!(
        resolve_import_target("nivela_core::media"),
        "nivela_core::media"
    );
    assert_eq!(resolve_import_target("nivela_core"), "nivela_core");
}
