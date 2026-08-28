// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Reusable, presentation-only UI building blocks shared across screens: pill "tag" chips,
//! section headers, card frames, generic enum combo boxes, and the clip-properties "labeled
//! section" wrapper. None of these read `App` — callers pass in already-translated text and
//! own mutable state, which keeps every component free to move, reuse, or swap independent of
//! app state and of each other.

mod combo;
mod frame;
mod property;
mod section;
mod tag;

pub use combo::enum_combo;
pub use frame::card_frame;
pub use property::{property_block, property_row, property_section, property_toggle};
pub use section::{modal_title, page_title, section_label};
pub use tag::{tag_accent, tag_error, tag_outline};
