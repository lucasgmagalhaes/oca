//! Workspace automation tasks, run via `cargo run -p xtask -- <command>` (or `make graph`,
//! `make docs`, etc. — see the root `Makefile`).
//!
//! ## `graph`
//!
//! Regenerates `docs/CODE_GRAPH.md`: a Mermaid module-dependency graph plus a per-module
//! index of every `pub` item and its doc summary, built by scanning `crates/*/src/**/*.rs`.
//!
//! This is a line-scanner, not a real Rust parser — it looks for `pub fn`/`pub struct`/etc.
//! at the start of a (trimmed) line and for `mod`/`use` declarations, which covers this
//! codebase's style well enough to be useful without pulling in a syn-based dependency. The
//! point isn't perfect accuracy, it's giving a future reader (human or AI) a map of "what's
//! here and how it's wired together" without having to open every file.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let command = std::env::args().nth(1).unwrap_or_default();
    match command.as_str() {
        "graph" => cmd_graph(),
        other => {
            eprintln!("unknown xtask command: {other:?}");
            eprintln!("available commands: graph");
            std::process::exit(1);
        }
    }
}

struct Item {
    kind: &'static str,
    name: String,
    doc: Option<String>,
}

struct Module {
    path: String,
    file: String,
    items: Vec<Item>,
}

fn cmd_graph() {
    let root = workspace_root();
    let crates_dir = root.join("crates");

    let mut modules = Vec::new();
    let mut edges: BTreeSet<(String, String, &'static str)> = BTreeSet::new();

    let mut crate_dirs: Vec<PathBuf> = fs::read_dir(&crates_dir)
        .expect("crates/ directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    crate_dirs.sort();

    for crate_dir in &crate_dirs {
        let crate_name = crate_dir.file_name().unwrap().to_string_lossy().replace('-', "_");
        let src_dir = crate_dir.join("src");
        if !src_dir.is_dir() {
            continue;
        }

        let mut rs_files = Vec::new();
        collect_rs_files(&src_dir, &mut rs_files);
        rs_files.sort();

        for file in &rs_files {
            let rel = file.strip_prefix(&src_dir).unwrap();
            let module_path = module_path_for(&crate_name, rel);
            let source = fs::read_to_string(file).unwrap_or_default();
            let items = extract_items(&source);
            for (kind, target) in extract_edges(&source, &crate_name, &module_path) {
                // `#[cfg(test)] mod tests` is inline test code, not an architectural
                // submodule — every file has one, so graphing it is pure noise.
                if kind == "declares" && target.ends_with("::tests") {
                    continue;
                }
                edges.insert((module_path.clone(), target, kind));
            }

            modules.push(Module {
                path: module_path,
                file: format!(
                    "{}/{}",
                    crate_dir.file_name().unwrap().to_string_lossy(),
                    rel.display().to_string().replace('\\', "/")
                ),
                items,
            });
        }
    }

    // `use`-edge targets are resolved with a "drop the trailing item if it's UpperCamelCase"
    // heuristic (see `resolve_relative`), which can't tell a lowercase *function* import
    // (`use nivela_core::media::format_timecode`) from a module import. Now that every real
    // module path is known, walk each unresolved "uses" target up its `::` segments until it
    // lands on one, correcting exactly that case.
    let known_modules: BTreeSet<String> = modules.iter().map(|m| m.path.clone()).collect();
    let edges: BTreeSet<(String, String, &'static str)> = edges
        .into_iter()
        .map(|(from, target, kind)| {
            if kind == "uses" && !known_modules.contains(&target) {
                (from, closest_known_ancestor(&target, &known_modules), kind)
            } else {
                (from, target, kind)
            }
        })
        .collect();

    let out = render_markdown(&modules, &edges);
    let docs_dir = root.join("docs");
    fs::create_dir_all(&docs_dir).expect("failed to create docs/");
    let out_path = docs_dir.join("CODE_GRAPH.md");
    fs::write(&out_path, out).expect("failed to write docs/CODE_GRAPH.md");
    println!("wrote {}", out_path.display());
}

fn workspace_root() -> PathBuf {
    // `cargo run -p xtask` sets CWD to the workspace root; fall back to walking up from the
    // binary's manifest dir (via CARGO_MANIFEST_DIR at compile time) if invoked some other way.
    let cwd = std::env::current_dir().expect("cwd");
    if cwd.join("Cargo.toml").exists() && cwd.join("crates").is_dir() {
        return cwd;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask should live at <root>/crates/xtask")
        .to_path_buf()
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Maps a source file's path (relative to its crate's `src/`) to a `crate::module::path`
/// string, e.g. `screens/mod.rs` -> `nivela_app::screens`, `probe.rs` -> `nivela_core::probe`.
fn module_path_for(crate_name: &str, rel_path: &Path) -> String {
    let mut segments: Vec<String> = rel_path
        .with_extension("")
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();

    if segments == ["lib"] || segments == ["main"] {
        return crate_name.to_string();
    }
    if segments.last().map(String::as_str) == Some("mod") {
        segments.pop();
    }

    let mut path = crate_name.to_string();
    for seg in segments {
        path.push_str("::");
        path.push_str(&seg);
    }
    path
}

fn extract_items(source: &str) -> Vec<Item> {
    const PREFIXES: &[(&str, &str)] = &[
        ("pub fn ", "fn"),
        ("pub const fn ", "fn"),
        ("pub async fn ", "fn"),
        ("pub struct ", "struct"),
        ("pub enum ", "enum"),
        ("pub trait ", "trait"),
        ("pub type ", "type"),
        ("pub const ", "const"),
    ];

    let lines: Vec<&str> = source.lines().collect();
    let mut items = Vec::new();

    for (i, raw_line) in lines.iter().enumerate() {
        let line = raw_line.trim_start();
        let Some((_, kind)) = PREFIXES.iter().find(|(prefix, _)| line.starts_with(prefix)) else {
            continue;
        };

        let name = extract_name(line);
        if name.is_empty() {
            continue;
        }

        // Walk upward over contiguous `///` doc lines directly above this item.
        let mut doc_lines = Vec::new();
        let mut j = i;
        while j > 0 {
            let prev = lines[j - 1].trim_start();
            if let Some(text) = prev.strip_prefix("///") {
                doc_lines.push(text.trim().to_string());
                j -= 1;
            } else {
                break;
            }
        }
        doc_lines.reverse();
        let doc = doc_lines.first().cloned().filter(|s| !s.is_empty());

        items.push(Item { kind, name, doc });
    }

    items
}

/// Pulls the identifier out of a `pub fn name(...)` / `pub struct Name` / etc. line.
fn extract_name(line: &str) -> String {
    let rest = line
        .trim_start_matches("pub ")
        .trim_start_matches("const ")
        .trim_start_matches("async ")
        .trim_start_matches("fn ")
        .trim_start_matches("struct ")
        .trim_start_matches("enum ")
        .trim_start_matches("trait ")
        .trim_start_matches("type ");
    rest.chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Coarse `mod`/`use` scan: returns `(kind, target_module_path)` pairs. `kind` is currently
/// unused by the caller beyond documenting intent, kept for readability at call sites.
fn extract_edges(source: &str, crate_name: &str, current_module: &str) -> Vec<(&'static str, String)> {
    let mut edges = Vec::new();

    for raw_line in source.lines() {
        let line = raw_line.trim_start();

        if let Some(rest) = line.strip_prefix("pub mod ").or_else(|| line.strip_prefix("mod ")) {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                edges.push(("declares", format!("{current_module}::{name}")));
            }
            continue;
        }

        let Some(rest) = line.strip_prefix("use ") else { continue };
        let path = rest.trim_end_matches(';').trim();
        let path = path.split_once("::{").map(|(base, _)| base).unwrap_or(path);
        let path = path.strip_suffix("::*").unwrap_or(path);

        let resolved = if let Some(rest) = path.strip_prefix("crate::") {
            resolve_relative(crate_name, rest)
        } else if let Some(rest) = path.strip_prefix("super::") {
            let parent = current_module.rsplit_once("::").map(|(p, _)| p).unwrap_or(crate_name);
            resolve_relative(parent, rest)
        } else if let Some(rest) = path.strip_prefix("self::") {
            resolve_relative(current_module, rest)
        } else if path.starts_with(crate_name) || path.starts_with("nivela_core") || path.starts_with("nivela_app") {
            resolve_import_target(path)
        } else {
            continue; // std/external crate — not interesting for this workspace graph.
        };

        if !resolved.is_empty() && resolved != current_module {
            edges.push(("uses", resolved));
        }
    }

    edges
}

/// Resolves a (prefix-stripped) absolute import path to the module it points at.
///
/// Rust convention makes types/traits `UpperCamelCase` and modules/functions `snake_case`,
/// so `a::b::Item` -> `a::b` (drop the trailing item) but `a::b::submodule` is left alone
/// (it's a module import, not an item import) — e.g. `nivela_core::media::MediaAsset`
/// resolves to `nivela_core::media`, while `nivela_core::media` stays as-is.
fn resolve_import_target(path: &str) -> String {
    match path.rsplit_once("::") {
        Some((head, tail)) if tail.starts_with(|c: char| c.is_ascii_uppercase()) => head.to_string(),
        _ => path.to_string(),
    }
}

/// Walks `path` up its `::` segments until it matches a real module in `known`, or returns
/// `path` unchanged if nothing along the way matches (e.g. it truly is an unresolvable edge).
fn closest_known_ancestor(path: &str, known: &BTreeSet<String>) -> String {
    let mut candidate = path;
    loop {
        if known.contains(candidate) {
            return candidate.to_string();
        }
        match candidate.rsplit_once("::") {
            Some((head, _)) => candidate = head,
            None => return path.to_string(),
        }
    }
}

/// Same idea as [`resolve_import_target`], but for a `parent`-relative import (the bit after
/// `crate::`/`super::`/`self::`) — handles the case where `rest` is a single bare identifier
/// too, e.g. `super::NivelaApp` resolves to just `parent` (the item lives directly in it),
/// while `super::widgets` resolves to `parent::widgets` (a sibling module).
fn resolve_relative(parent: &str, rest: &str) -> String {
    match rest.rsplit_once("::") {
        Some((head, tail)) if tail.starts_with(|c: char| c.is_ascii_uppercase()) => {
            format!("{parent}::{head}")
        }
        Some(_) => format!("{parent}::{rest}"),
        None if rest.starts_with(|c: char| c.is_ascii_uppercase()) => parent.to_string(),
        None => format!("{parent}::{rest}"),
    }
}

fn render_markdown(modules: &[Module], edges: &BTreeSet<(String, String, &'static str)>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# NivelaEditor — code graph");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Auto-generated by `cargo run -p xtask -- graph` (or `make graph`). Do not edit by \
         hand — it will be overwritten. Regenerate after adding/removing modules or public \
         items so this stays a reliable map for exploring the codebase without re-reading \
         every file."
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## Module graph");
    let _ = writeln!(out);
    let _ = writeln!(out, "Solid arrows are `mod` declarations (structural); dashed arrows are `use` edges (a module reads from another).");
    let _ = writeln!(out);
    let _ = writeln!(out, "```mermaid");
    let _ = writeln!(out, "graph LR");
    for (from, to, kind) in edges {
        let arrow = if *kind == "declares" { "-->" } else { "-.->" };
        let _ = writeln!(out, "    {from:?} {arrow} {to:?}");
    }
    let _ = writeln!(out, "```");
    let _ = writeln!(out);

    let _ = writeln!(out, "## Module index");
    let _ = writeln!(out);
    for module in modules {
        let _ = writeln!(out, "### `{}`", module.path);
        let _ = writeln!(out, "*{}*", module.file);
        let _ = writeln!(out);
        if module.items.is_empty() {
            let _ = writeln!(out, "_No public items._");
        } else {
            for item in &module.items {
                match &item.doc {
                    Some(doc) => {
                        let _ = writeln!(out, "- **{}** `{}` — {}", item.kind, item.name, doc);
                    }
                    None => {
                        let _ = writeln!(out, "- **{}** `{}`", item.kind, item.name);
                    }
                }
            }
        }
        let _ = writeln!(out);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_path_for_lib_root_is_the_crate_name() {
        assert_eq!(module_path_for("nivela_core", Path::new("lib.rs")), "nivela_core");
    }

    #[test]
    fn module_path_for_a_plain_file_appends_its_stem() {
        assert_eq!(module_path_for("nivela_core", Path::new("probe.rs")), "nivela_core::probe");
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
    fn ignores_private_items() {
        let src = "fn helper() {}\nstruct Internal;\n";
        assert!(extract_items(src).is_empty());
    }

    #[test]
    fn resolves_crate_relative_use_edges() {
        let edges = extract_edges("use crate::media::MediaAsset;\n", "nivela_core", "nivela_core::probe");
        assert!(edges.contains(&("uses", "nivela_core::media".to_string())));
    }

    #[test]
    fn resolves_super_relative_use_edges() {
        // `widgets` is a sibling module (lowercase), so it's kept as the target rather than
        // treated as an item to drop.
        let edges = extract_edges("use super::widgets;\n", "nivela_app", "nivela_app::screens::home");
        assert!(edges.contains(&("uses", "nivela_app::screens::widgets".to_string())));
    }

    #[test]
    fn resolves_super_relative_use_edges_for_an_item() {
        let edges = extract_edges("use super::NivelaApp;\n", "nivela_app", "nivela_app::screens::home");
        assert!(edges.contains(&("uses", "nivela_app::screens".to_string())));
    }

    #[test]
    fn tracks_declared_submodules() {
        let edges = extract_edges("pub mod probe;\n", "nivela_core", "nivela_core");
        assert!(edges.contains(&("declares", "nivela_core::probe".to_string())));
    }

    #[test]
    fn resolve_import_target_drops_a_trailing_item_but_keeps_a_trailing_module() {
        assert_eq!(resolve_import_target("nivela_core::media::MediaAsset"), "nivela_core::media");
        assert_eq!(resolve_import_target("nivela_core::media"), "nivela_core::media");
        assert_eq!(resolve_import_target("nivela_core"), "nivela_core");
    }
}
