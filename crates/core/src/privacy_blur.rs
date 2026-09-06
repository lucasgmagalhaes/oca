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

//! CF-09 (`spec/architecture/competitive-feature-plan.md`)'s slice 4 — "Support privacy blur as
//! the first end-to-end effect before general selective effects." A thin `avcore` wrapper over
//! `avbridge::apply_privacy_blur`'s new post-process pass (same "opens an already-rendered
//! export, writes a modified copy" shape `avcore::export`'s existing text/shape overlay
//! composition already uses), which blurs an already-rendered export everywhere a matte video's
//! own luma is non-zero.
//!
//! Ties together this session's two other CF-09 pieces: [`crate::mask_propagation::
//! rasterize_to_matte_frames`] turns a propagated arbitrary-object mask into per-frame matte
//! buffers, [`crate::background_removal::encode_matte_video`] (reused — not a second matte
//! encoder) turns those into a matte video file, and [`apply_privacy_blur`] here consumes that
//! file to actually blur the tracked region. **Real end-to-end runtime behavior is unverified in
//! this sandbox** — this repo's own too-old packaged FFmpeg build can't compile the rest of
//! `avbridge` at all (see `CLAUDE.md`), so this new C function has only been syntax-checked
//! per-file (`gcc -fsyntax-only`) against the real FFmpeg headers, never actually run against a
//! real video. Treat the filter-graph string itself (`gblur`+`maskedmerge`, see `privacy_blur.c`)
//! as code-reviewed, not execution-verified.

use std::path::Path;

/// What [`apply_privacy_blur`] failed on — thin wrapper over [`avbridge::PrivacyBlurError`],
/// same pattern [`crate::background_removal::MatteEncodeError`] already uses for
/// `avbridge::MatteError`.
#[derive(Debug)]
pub enum PrivacyBlurError {
    Bridge(avbridge::PrivacyBlurError),
}

impl std::fmt::Display for PrivacyBlurError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivacyBlurError::Bridge(e) => write!(f, "failed to apply privacy blur: {e}"),
        }
    }
}

impl std::error::Error for PrivacyBlurError {}

/// Blurs `in_path` (an already-rendered export) everywhere `matte_path`'s own per-frame luma is
/// non-zero, writing the result to `out_path`. See [`avbridge::apply_privacy_blur`]'s doc
/// comment for the exact filter graph and the `matte_path`/timing contract (it must already be
/// encoded at `canvas_fps_num`/`_den`, e.g. via [`crate::background_removal::
/// encode_matte_video`]).
#[allow(clippy::too_many_arguments)]
pub fn apply_privacy_blur(
    in_path: &Path,
    out_path: &Path,
    matte_path: &Path,
    blur_sigma: f64,
    canvas_width: u32,
    canvas_height: u32,
    fps_num: u32,
    fps_den: u32,
) -> Result<(), PrivacyBlurError> {
    avbridge::apply_privacy_blur(
        in_path,
        out_path,
        matte_path,
        blur_sigma,
        canvas_width,
        canvas_height,
        fps_num,
        fps_den,
    )
    .map_err(PrivacyBlurError::Bridge)
}
