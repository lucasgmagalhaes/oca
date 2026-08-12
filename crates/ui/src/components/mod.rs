//! Reusable, presentation-only UI building blocks shared across screens: pill "tag" chips,
//! section headers, card frames, generic enum combo boxes, and the clip-properties "labeled
//! section" wrapper. None of these read `OcaApp` — callers pass in already-translated text and
//! own mutable state, which keeps every component free to move, reuse, or swap independent of
//! app state and of each other.

mod combo;
mod frame;
mod property;
mod section;
mod tag;

pub use combo::enum_combo;
pub use frame::card_frame;
pub use property::{property_block, property_section, property_toggle};
pub use section::section_label;
pub use tag::{tag_accent, tag_error, tag_outline};
