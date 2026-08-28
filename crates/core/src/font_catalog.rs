// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! FONT-01A: the locked built-in font catalog manifest
//! ([`spec/architecture/built-in-font-catalog.md`](../../../../spec/architecture/built-in-font-catalog.md)).
//!
//! Six families today, matching [`crate::timeline::TextFontFamily`]'s current enum variants
//! one-to-one via [`TextFontFamily::family_id`]/[`TextFontFamily::from_family_id`]. The manifest
//! fields mirror the doc's required typed fields (`family_id`, `source_path`, `sha256`, license,
//! weights, ...) so a later slice (FONT-01B) can grow this to the full 43-family/51-file catalog
//! without a new format migration — this slice is the manifest and its self-consistency
//! validation only, not the catalog expansion or the persisted-identity swap on `TextClip`
//! (`.ocproj` still stores the plain enum variant name; see that struct's own doc comment).
//!
//! Static data only. Acquisition/vendoring is a maintainer-time, offline operation per the doc's
//! "Acquisition and update workflow" section — no network code belongs in this module, and none
//! exists here.

use crate::timeline::TextFontFamily;

/// Revision of the pinned `google/fonts` upstream repository this catalog's binaries were
/// vendored from, per the doc's "Measured package budget" section.
pub const GOOGLE_FONTS_REVISION: &str = "ade3d1533e06b2b1462ffcde8e08b129627ca360";

/// Schema version of this manifest's own shape. Bump only when a field's meaning changes, not
/// when a family is added.
pub const CATALOG_VERSION: u32 = 1;

/// Curated grouping used by the font selector's category sections (doc: "sans, display, serif,
/// handwritten, monospace").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontCategory {
    Sans,
    Display,
    Serif,
    Handwritten,
    Monospace,
}

/// Whether a family's upstream binaries are static per-weight files or one variable-axis file.
/// Every family in this initial six-family manifest is static; `Variable` exists so FONT-01B's
/// variable families (`Inter[opsz,wght].ttf` and friends) slot into the same shape without a
/// breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSourceKind {
    Static,
    #[allow(dead_code)]
    Variable,
}

/// One locked, on-disk font binary within a family — a static weight/style file today; a single
/// variable-axis file (one entry covering every weight) once FONT-01B lands those.
#[derive(Debug, Clone, Copy)]
pub struct FontFace {
    /// Numeric weight this face renders as (400 = Regular, 700 = Bold, ...).
    pub weight: u16,
    /// Path to the vendored binary, relative to `crates/core/assets/fonts/`.
    pub source_path: &'static str,
    /// Lowercase hex SHA-256 of the exact bytes at `source_path`.
    pub sha256: &'static str,
    pub size_bytes: u64,
}

/// One catalog family — the manifest unit the doc describes, keyed by a stable `family_id` slug
/// that is never localized and never changes meaning once shipped.
#[derive(Debug, Clone, Copy)]
pub struct FontFamilyEntry {
    /// Stable ASCII slug, e.g. `"lato"`. This is the catalog's persisted identity target
    /// (FONT-01A's remaining `.ocproj` migration, not yet wired) — never a translated
    /// `display_name`, never an enum ordinal.
    pub family_id: &'static str,
    pub display_name: &'static str,
    pub category: FontCategory,
    pub tags: &'static [&'static str],
    /// Path to the vendored `OFL.txt`, relative to `crates/core/assets/fonts/`.
    pub license_path: &'static str,
    pub license_id: &'static str,
    pub source_kind: FontSourceKind,
    /// One or more locked binaries. Every weight in a static family's own file set; exactly one
    /// entry for a variable family (FONT-01B).
    pub faces: &'static [FontFace],
    /// Weight selected when a project doesn't otherwise specify one.
    pub default_weight: u16,
    /// `family_id` of the deterministic fallback used when this family's own bytes fail to
    /// parse (doc: "Lato Regular for GF Latin Core characters missing because of a
    /// corrupt/invalid selection"). `None` only for the fallback family itself.
    pub fallback_family_id: Option<&'static str>,
    /// Coarse glyph-coverage claims this entry backs. `"gf-latin-core"` covers the acceptance
    /// corpus in the doc's "Shaping and script support" section (Portuguese/Spanish/English).
    pub glyphset_guarantees: &'static [&'static str],
}

const GF_LATIN_CORE: &[&str] = &["gf-latin-core"];

/// The locked catalog: today's six bundled families, one entry each. Order matches
/// [`TextFontFamily::ALL`] and the asset README's table.
pub const CATALOG: &[FontFamilyEntry] = &[
    FontFamilyEntry {
        family_id: "lato",
        display_name: "Lato",
        category: FontCategory::Sans,
        tags: &["caption", "general"],
        license_path: "lato/OFL.txt",
        license_id: "OFL-1.1",
        source_kind: FontSourceKind::Static,
        faces: &[
            FontFace {
                weight: 400,
                source_path: "lato/Lato-Regular.ttf",
                sha256: "d636e4683231f931eda222d588e944d082bfd3bdba02f928bee461c0f185b251",
                size_bytes: 656_568,
            },
            FontFace {
                weight: 700,
                source_path: "lato/Lato-Bold.ttf",
                sha256: "8a0aace75d33794eece4b28187bfc1df0bbd2888b5d8a56e01788c8d65d16be1",
                size_bytes: 656_544,
            },
        ],
        default_weight: 400,
        fallback_family_id: None,
        glyphset_guarantees: GF_LATIN_CORE,
    },
    FontFamilyEntry {
        family_id: "bebas-neue",
        display_name: "Bebas Neue",
        category: FontCategory::Display,
        tags: &["condensed", "title"],
        license_path: "bebas-neue/OFL.txt",
        license_id: "OFL-1.1",
        source_kind: FontSourceKind::Static,
        faces: &[FontFace {
            weight: 400,
            source_path: "bebas-neue/BebasNeue-Regular.ttf",
            sha256: "08e4623805102d819f58601e46e345648846075e363b2ceb23313c2d1c83ec73",
            size_bytes: 61_400,
        }],
        default_weight: 400,
        fallback_family_id: Some("lato"),
        glyphset_guarantees: GF_LATIN_CORE,
    },
    FontFamilyEntry {
        family_id: "playfair-display-sc",
        display_name: "Playfair Display SC",
        category: FontCategory::Serif,
        tags: &["serif", "high-contrast"],
        license_path: "playfair-display-sc/OFL.txt",
        license_id: "OFL-1.1",
        source_kind: FontSourceKind::Static,
        faces: &[
            FontFace {
                weight: 400,
                source_path: "playfair-display-sc/PlayfairDisplaySC-Regular.ttf",
                sha256: "55df09f0a49905a0f5e478597b5b3fc78acf0512ceb568504f3cdc4659662e47",
                size_bytes: 201_772,
            },
            FontFace {
                weight: 700,
                source_path: "playfair-display-sc/PlayfairDisplaySC-Bold.ttf",
                sha256: "b58a968cfc9b4e96e6857a206e1b042dc3fdbe5dc31cd75825d864f80ac7e915",
                size_bytes: 204_568,
            },
        ],
        default_weight: 400,
        fallback_family_id: Some("lato"),
        glyphset_guarantees: GF_LATIN_CORE,
    },
    FontFamilyEntry {
        family_id: "patrick-hand",
        display_name: "Patrick Hand",
        category: FontCategory::Handwritten,
        tags: &["handwritten", "casual"],
        license_path: "patrick-hand/OFL.txt",
        license_id: "OFL-1.1",
        source_kind: FontSourceKind::Static,
        faces: &[FontFace {
            weight: 400,
            source_path: "patrick-hand/PatrickHand-Regular.ttf",
            sha256: "0f173b3e6cb6d1af25babf7f0057c5ac4ee11f9992b0469bb817e967ef4ad0fc",
            size_bytes: 214_772,
        }],
        default_weight: 400,
        fallback_family_id: Some("lato"),
        glyphset_guarantees: GF_LATIN_CORE,
    },
    FontFamilyEntry {
        family_id: "anonymous-pro",
        display_name: "Anonymous Pro",
        category: FontCategory::Monospace,
        tags: &["monospace", "code"],
        license_path: "anonymous-pro/OFL.txt",
        license_id: "OFL-1.1",
        source_kind: FontSourceKind::Static,
        faces: &[
            FontFace {
                weight: 400,
                source_path: "anonymous-pro/AnonymousPro-Regular.ttf",
                sha256: "46d8b9a5f4b38fc9d30f3cdd676d4c6f78a9bef949bb1a8304216cc731eb87f8",
                size_bytes: 158_100,
            },
            FontFace {
                weight: 700,
                source_path: "anonymous-pro/AnonymousPro-Bold.ttf",
                sha256: "52895657e7d48860089ebfac63a27244f37a2218ec9c5019a0b37fe885c0fc0c",
                size_bytes: 153_616,
            },
        ],
        default_weight: 400,
        fallback_family_id: Some("lato"),
        glyphset_guarantees: GF_LATIN_CORE,
    },
    FontFamilyEntry {
        family_id: "archivo-black",
        display_name: "Archivo Black",
        category: FontCategory::Display,
        tags: &["bold", "caption"],
        license_path: "archivo-black/OFL.txt",
        license_id: "OFL-1.1",
        source_kind: FontSourceKind::Static,
        faces: &[FontFace {
            weight: 400,
            source_path: "archivo-black/ArchivoBlack-Regular.ttf",
            sha256: "dd9a89a019b4849f66ab75455fe7bdf931311042cbb0f0f97acc061539703180",
            size_bytes: 90_988,
        }],
        default_weight: 400,
        fallback_family_id: Some("lato"),
        glyphset_guarantees: GF_LATIN_CORE,
    },
];

/// Looks up one family by its stable slug.
pub fn find_family(family_id: &str) -> Option<&'static FontFamilyEntry> {
    CATALOG.iter().find(|entry| entry.family_id == family_id)
}

/// Self-consistency checks over [`CATALOG`] alone — no file I/O, so this can run unconditionally
/// (e.g. from a unit test or a future startup smoke check) without touching disk. Bundled-byte
/// verification (hash/size against the real files) is a separate, `include_bytes!`-driven test
/// in this module's own test submodule, since `include_bytes!` needs a literal path per call.
pub fn validate_static_catalog() -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut seen_family_ids = std::collections::HashSet::new();
    let mut seen_source_paths = std::collections::HashSet::new();

    for entry in CATALOG {
        if entry.family_id.is_empty()
            || !entry
                .family_id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            errors.push(format!(
                "family_id {:?} must be a non-empty lowercase ASCII slug",
                entry.family_id
            ));
        }
        if !seen_family_ids.insert(entry.family_id) {
            errors.push(format!("duplicate family_id {:?}", entry.family_id));
        }
        if entry.faces.is_empty() {
            errors.push(format!("family {:?} declares no faces", entry.family_id));
        }
        if !entry
            .faces
            .iter()
            .any(|face| face.weight == entry.default_weight)
        {
            errors.push(format!(
                "family {:?} default_weight {} has no matching face",
                entry.family_id, entry.default_weight
            ));
        }
        if entry.license_id.is_empty() || entry.license_path.is_empty() {
            errors.push(format!(
                "family {:?} is missing license metadata",
                entry.family_id
            ));
        }
        if let Some(fallback) = entry.fallback_family_id {
            if fallback == entry.family_id {
                errors.push(format!(
                    "family {:?} cannot be its own fallback",
                    entry.family_id
                ));
            } else if !CATALOG.iter().any(|other| other.family_id == fallback) {
                errors.push(format!(
                    "family {:?} declares unknown fallback_family_id {:?}",
                    entry.family_id, fallback
                ));
            }
        }
        for face in entry.faces {
            if face.sha256.len() != 64 || !face.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
                errors.push(format!(
                    "family {:?} face {:?} has a malformed sha256",
                    entry.family_id, face.source_path
                ));
            }
            if !seen_source_paths.insert(face.source_path) {
                errors.push(format!("duplicate source_path {:?}", face.source_path));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

impl TextFontFamily {
    /// This family's stable catalog slug (`"lato"`, `"bebas-neue"`, ...) — the identity FONT-01
    /// intends `.ocproj` to eventually persist instead of the enum variant name. Every
    /// [`TextFontFamily::ALL`] member always resolves to a real [`CATALOG`] entry; a debug build
    /// panics if that invariant is ever broken (guarded by this module's own tests in release
    /// builds too, so the mismatch is caught long before it ships).
    pub fn family_id(self) -> &'static str {
        let id = match self {
            Self::Lato => "lato",
            Self::BebasNeue => "bebas-neue",
            Self::PlayfairDisplay => "playfair-display-sc",
            Self::PatrickHand => "patrick-hand",
            Self::AnonymousPro => "anonymous-pro",
            Self::ArchivoBlack => "archivo-black",
        };
        debug_assert!(
            find_family(id).is_some(),
            "TextFontFamily variant has no matching font_catalog entry: {id}"
        );
        id
    }

    /// Reverse of [`Self::family_id`]. Returns `None` for a slug this build doesn't recognize —
    /// per the doc's forwards-compatibility rule, callers must keep the raw id and fall back to
    /// [`Self::default`] for rendering rather than treating an unknown id as corruption.
    pub fn from_family_id(family_id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|family| family.family_id() == family_id)
    }
}

#[cfg(test)]
#[path = "font_catalog/font_catalog_test.rs"]
mod tests;
