use super::*;
use sha2::{Digest, Sha256};

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn static_catalog_is_self_consistent() {
    assert_eq!(validate_static_catalog(), Ok(()));
}

#[test]
fn every_text_font_family_variant_has_a_catalog_entry() {
    for family in TextFontFamily::ALL {
        let id = family.family_id();
        let entry = find_family(id).unwrap_or_else(|| panic!("no catalog entry for {id:?}"));
        assert_eq!(entry.family_id, id);
    }
}

#[test]
fn family_id_round_trips_through_from_family_id() {
    for family in TextFontFamily::ALL {
        assert_eq!(
            TextFontFamily::from_family_id(family.family_id()),
            Some(family)
        );
    }
}

#[test]
fn from_family_id_rejects_unknown_slugs() {
    assert_eq!(TextFontFamily::from_family_id("comic-sans"), None);
    assert_eq!(TextFontFamily::from_family_id(""), None);
}

#[test]
fn every_fallback_chain_terminates_at_lato_within_one_hop() {
    // A stronger property than validate_static_catalog()'s own "fallback id exists" check:
    // the doc's fallback policy is "Lato Regular for GF Latin Core characters missing because
    // of a corrupt/invalid selection" specifically, not an arbitrary chain.
    for entry in CATALOG {
        match entry.fallback_family_id {
            None => assert_eq!(entry.family_id, "lato"),
            Some(fallback) => assert_eq!(fallback, "lato"),
        }
    }
}

// Bundled-byte verification: include_bytes! needs a literal path per call site, so this can't
// loop over CATALOG's own source_path strings — each family's faces are checked individually
// against the exact bytes compiled into the binary, catching a manifest hash/size that has
// drifted from the actual vendored file (the doc's "Verify SHA-256" requirement) for real, not
// just a manifest self-consistency check.

fn assert_face_matches(family_id: &str, source_path: &str, bytes: &[u8]) {
    let entry = find_family(family_id).expect("family missing from catalog");
    let face = entry
        .faces
        .iter()
        .find(|f| f.source_path == source_path)
        .unwrap_or_else(|| panic!("no face {source_path:?} on family {family_id:?}"));
    assert_eq!(
        face.size_bytes,
        bytes.len() as u64,
        "size_bytes drifted for {source_path}"
    );
    assert_eq!(
        face.sha256,
        hex_sha256(bytes),
        "sha256 drifted for {source_path}"
    );
}

#[test]
fn locked_face_bytes_has_exactly_one_entry_per_catalog_face() {
    let expected: usize = CATALOG.iter().map(|entry| entry.faces.len()).sum();
    assert_eq!(locked_face_bytes().len(), expected);
    for entry in CATALOG {
        for face in entry.faces {
            let bytes = face_bytes(entry.family_id, face.weight).unwrap_or_else(|| {
                panic!(
                    "locked_face_bytes() has no entry for ({}, {})",
                    entry.family_id, face.weight
                )
            });
            assert_eq!(bytes.len() as u64, face.size_bytes);
            assert_eq!(hex_sha256(bytes), face.sha256);
        }
    }
}

#[test]
fn face_bytes_returns_none_for_an_unknown_family_or_weight() {
    assert!(face_bytes("comic-sans", 400).is_none());
    assert!(face_bytes("lato", 900).is_none());
}

#[test]
fn bundled_lato_bytes_match_the_locked_manifest() {
    assert_face_matches(
        "lato",
        "lato/Lato-Regular.ttf",
        include_bytes!("../../assets/fonts/lato/Lato-Regular.ttf"),
    );
    assert_face_matches(
        "lato",
        "lato/Lato-Bold.ttf",
        include_bytes!("../../assets/fonts/lato/Lato-Bold.ttf"),
    );
}

#[test]
fn bundled_bebas_neue_bytes_match_the_locked_manifest() {
    assert_face_matches(
        "bebas-neue",
        "bebas-neue/BebasNeue-Regular.ttf",
        include_bytes!("../../assets/fonts/bebas-neue/BebasNeue-Regular.ttf"),
    );
}

#[test]
fn bundled_playfair_display_sc_bytes_match_the_locked_manifest() {
    assert_face_matches(
        "playfair-display-sc",
        "playfair-display-sc/PlayfairDisplaySC-Regular.ttf",
        include_bytes!("../../assets/fonts/playfair-display-sc/PlayfairDisplaySC-Regular.ttf"),
    );
    assert_face_matches(
        "playfair-display-sc",
        "playfair-display-sc/PlayfairDisplaySC-Bold.ttf",
        include_bytes!("../../assets/fonts/playfair-display-sc/PlayfairDisplaySC-Bold.ttf"),
    );
}

#[test]
fn bundled_patrick_hand_bytes_match_the_locked_manifest() {
    assert_face_matches(
        "patrick-hand",
        "patrick-hand/PatrickHand-Regular.ttf",
        include_bytes!("../../assets/fonts/patrick-hand/PatrickHand-Regular.ttf"),
    );
}

#[test]
fn bundled_anonymous_pro_bytes_match_the_locked_manifest() {
    assert_face_matches(
        "anonymous-pro",
        "anonymous-pro/AnonymousPro-Regular.ttf",
        include_bytes!("../../assets/fonts/anonymous-pro/AnonymousPro-Regular.ttf"),
    );
    assert_face_matches(
        "anonymous-pro",
        "anonymous-pro/AnonymousPro-Bold.ttf",
        include_bytes!("../../assets/fonts/anonymous-pro/AnonymousPro-Bold.ttf"),
    );
}

#[test]
fn bundled_archivo_black_bytes_match_the_locked_manifest() {
    assert_face_matches(
        "archivo-black",
        "archivo-black/ArchivoBlack-Regular.ttf",
        include_bytes!("../../assets/fonts/archivo-black/ArchivoBlack-Regular.ttf"),
    );
}

#[test]
fn license_and_source_paths_exist_on_disk_and_never_traverse() {
    // Doc's "static destination allowlist" concern: every path is a plain relative filename
    // under the fonts asset root, never `..` or absolute — and it actually resolves to a real
    // file, not just a plausible-looking string in the manifest.
    let assets_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts");
    for entry in CATALOG {
        assert!(
            !entry.license_path.contains("..")
                && !std::path::Path::new(entry.license_path).is_absolute(),
            "license_path must not traverse: {}",
            entry.license_path
        );
        assert!(
            assets_root.join(entry.license_path).is_file(),
            "license_path does not exist on disk: {}",
            entry.license_path
        );
        for face in entry.faces {
            assert!(
                !face.source_path.contains("..")
                    && !std::path::Path::new(face.source_path).is_absolute(),
                "source_path must not traverse: {}",
                face.source_path
            );
            assert!(
                assets_root.join(face.source_path).is_file(),
                "source_path does not exist on disk: {}",
                face.source_path
            );
        }
    }
}
