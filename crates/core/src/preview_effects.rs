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

//! P4 item 21, "Preview support for vignette/glitch/deflicker/3D-LUT/stabilization"
//! (`spec/ROADMAP.md`) — confirmed a hard wall for the *real* `avfilter` output (no matching
//! GStreamer element exists on any dev machine checked, per `matrix/effects-and-color.md`).
//! Rather than leave the gap entirely closed, this covers the two effects with a precise,
//! well-specified, non-temporal per-pixel definition -- a 3D LUT lookup and a radial vignette --
//! as **CPU-side post-processing of the already-decoded preview frame**, the same pattern
//! [`crate::scopes`] already established for the waveform/vectorscope overlays (post-process the
//! decoded `VideoFrame`, no new avfilter/GStreamer element). Applied only to `App`'s live preview
//! texture in `ui`; the real export path is untouched and still uses the exact `lut3d`/`vignette`
//! `avfilter`s (`ClipInstance::video_filter_chain`) for final output.
//!
//! **This is a preview approximation, not a bit-exact reproduction of either `avfilter`.** The
//! vignette formula here is a simple radial falloff, not FFmpeg's own cosine-based one (deriving
//! that exactly from `libavfilter`'s C source without being able to visually A/B preview against
//! export in this environment isn't a risk worth taking — see the roadmap item's own caution).
//! The LUT lookup, in contrast, *is* precise: the `.cube` format and trilinear interpolation are
//! both fully specified, deterministic, and unit-testable without needing a GUI.
//!
//! **Explicitly not covered by this pass** (left as the roadmap item's remaining gap): glitch
//! (there's no single well-specified "the" glitch algorithm to approximate — whatever look is
//! chosen is a judgment call this sandbox can't visually verify), deflicker and stabilization
//! (both need *temporal* state across multiple frames — a rolling frame history in `App` — a
//! materially larger, stateful piece of work with its own seek/scrub edge cases, not a natural
//! extension of this per-frame-only module).

use std::path::Path;

/// A parsed 3D LUT (`.cube` file, Adobe's Common LUT Format — the same format
/// [`crate::timeline::ClipInstance::lut_path`] already stores a path to for export). `size`
/// entries per axis, `data` flattened with red the fastest-varying index (`r + g*size +
/// b*size*size`), matching the `.cube` spec's own data ordering.
#[derive(Debug, Clone, PartialEq)]
pub struct Lut3D {
    size: usize,
    data: Vec<[f32; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LutParseError {
    MissingSize,
    InvalidSize,
    SizeTooSmall(i64),
    WrongRowCount { expected: usize, found: usize },
    MalformedRow(usize),
}

impl std::fmt::Display for LutParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LutParseError::MissingSize => write!(f, "no LUT_3D_SIZE line found"),
            LutParseError::InvalidSize => write!(f, "LUT_3D_SIZE isn't a valid integer"),
            LutParseError::SizeTooSmall(n) => {
                write!(f, "LUT_3D_SIZE must be at least 2, got {n}")
            }
            LutParseError::WrongRowCount { expected, found } => {
                write!(f, "expected {expected} data rows, found {found}")
            }
            LutParseError::MalformedRow(row) => {
                write!(f, "data row {row} doesn't have 3 numbers")
            }
        }
    }
}

impl std::error::Error for LutParseError {}

impl Lut3D {
    /// Parses a `.cube` file's contents. Tolerates `#`-comments, blank lines, and the optional
    /// `TITLE`/`DOMAIN_MIN`/`DOMAIN_MAX` metadata lines (the last two are read but not applied —
    /// this codebase's own exported LUTs, and every common vendor LUT, use the default `0..1`
    /// domain; a non-default domain would just mean the preview approximation is slightly off,
    /// never a crash or a panic).
    pub fn parse(contents: &str) -> Result<Self, LutParseError> {
        let mut size: Option<usize> = None;
        let mut data = Vec::new();

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("LUT_3D_SIZE") {
                let n: i64 = rest
                    .trim()
                    .parse()
                    .map_err(|_| LutParseError::InvalidSize)?;
                if n < 2 {
                    return Err(LutParseError::SizeTooSmall(n));
                }
                size = Some(n as usize);
                continue;
            }
            if line.starts_with("TITLE")
                || line.starts_with("DOMAIN_MIN")
                || line.starts_with("DOMAIN_MAX")
                || line.starts_with("LUT_1D_SIZE")
            {
                continue;
            }

            let values: Vec<f32> = line
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if values.len() != 3 {
                return Err(LutParseError::MalformedRow(data.len()));
            }
            data.push([values[0], values[1], values[2]]);
        }

        let size = size.ok_or(LutParseError::MissingSize)?;
        let expected = size * size * size;
        if data.len() != expected {
            return Err(LutParseError::WrongRowCount {
                expected,
                found: data.len(),
            });
        }
        Ok(Self { size, data })
    }

    /// Reads and parses `path`. Errors the same way [`std::fs::read_to_string`]/[`Self::parse`]
    /// would (I/O failure surfaces as a generic parse-shaped error via [`LutParseError::
    /// MissingSize`] — the caller only needs "did this work," not to distinguish the two).
    pub fn load(path: &Path) -> Result<Self, LutParseError> {
        let contents = std::fs::read_to_string(path).map_err(|_| LutParseError::MissingSize)?;
        Self::parse(&contents)
    }

    fn at(&self, x: usize, y: usize, z: usize) -> [f32; 3] {
        self.data[x + y * self.size + z * self.size * self.size]
    }

    /// Trilinear-interpolated lookup for a normalized `(r, g, b)` in `[0.0, 1.0]` (out-of-range
    /// inputs are clamped, never a panic or a wraparound).
    pub fn sample(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let max_index = (self.size - 1) as f32;
        let fx = r.clamp(0.0, 1.0) * max_index;
        let fy = g.clamp(0.0, 1.0) * max_index;
        let fz = b.clamp(0.0, 1.0) * max_index;

        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let z0 = fz.floor() as usize;
        let x1 = (x0 + 1).min(self.size - 1);
        let y1 = (y0 + 1).min(self.size - 1);
        let z1 = (z0 + 1).min(self.size - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let tz = fz - z0 as f32;

        let lerp3 = |a: [f32; 3], b: [f32; 3], t: f32| {
            [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]
        };

        let c00 = lerp3(self.at(x0, y0, z0), self.at(x1, y0, z0), tx);
        let c10 = lerp3(self.at(x0, y1, z0), self.at(x1, y1, z0), tx);
        let c01 = lerp3(self.at(x0, y0, z1), self.at(x1, y0, z1), tx);
        let c11 = lerp3(self.at(x0, y1, z1), self.at(x1, y1, z1), tx);
        let c0 = lerp3(c00, c10, ty);
        let c1 = lerp3(c01, c11, ty);
        lerp3(c0, c1, tz)
    }
}

/// Applies `lut` to `rgba` in place (a tightly-packed `width*height*4`-byte RGBA8 buffer, alpha
/// untouched) — what [`crate::timeline::ClipInstance::lut_path`] means for a live preview frame,
/// via [`Lut3D::sample`]. No-op if `rgba`'s length isn't a multiple of 4.
pub fn apply_lut_to_rgba(rgba: &mut [u8], lut: &Lut3D) {
    for pixel in rgba.chunks_exact_mut(4) {
        let [r, g, b] = lut.sample(
            pixel[0] as f32 / 255.0,
            pixel[1] as f32 / 255.0,
            pixel[2] as f32 / 255.0,
        );
        pixel[0] = (r.clamp(0.0, 1.0) * 255.0).round() as u8;
        pixel[1] = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
        pixel[2] = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

/// Applies a radial vignette darkening to `rgba` in place (a tightly-packed `width*height*4`-
/// byte RGBA8 buffer, alpha untouched) -- a **preview approximation**, not the real `vignette`
/// `avfilter`'s exact cosine falloff (see this module's doc comment). `intensity` in `[0.0,
/// 1.0]`-ish (matches [`crate::timeline::ClipInstance::vignette_intensity`]'s own range): `0.0`
/// is a no-op, `1.0` darkens the frame's extreme corners to black. No-op if `width`/`height` is
/// `0` or `rgba`'s length doesn't match `width*height*4`.
pub fn apply_vignette_to_rgba(rgba: &mut [u8], width: u32, height: u32, intensity: f32) {
    if width == 0 || height == 0 || intensity <= 0.0 {
        return;
    }
    if rgba.len() != (width as usize) * (height as usize) * 4 {
        return;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    // The corner-to-center distance in this normalized space, so `norm_dist` reaches exactly
    // 1.0 at every frame corner regardless of aspect ratio.
    let corner_dist = (2.0_f32).sqrt();

    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32 + 0.5 - cx) / cx;
            let dy = (y as f32 + 0.5 - cy) / cy;
            let norm_dist = ((dx * dx + dy * dy).sqrt() / corner_dist).min(1.0);
            let factor = 1.0 - intensity * norm_dist * norm_dist;
            let index = ((y * width + x) * 4) as usize;
            rgba[index] = (rgba[index] as f32 * factor).round().clamp(0.0, 255.0) as u8;
            rgba[index + 1] = (rgba[index + 1] as f32 * factor).round().clamp(0.0, 255.0) as u8;
            rgba[index + 2] = (rgba[index + 2] as f32 * factor).round().clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_cube(size: usize) -> String {
        let mut out = format!("LUT_3D_SIZE {size}\n");
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let denom = (size - 1) as f32;
                    out.push_str(&format!(
                        "{:.6} {:.6} {:.6}\n",
                        r as f32 / denom,
                        g as f32 / denom,
                        b as f32 / denom
                    ));
                }
            }
        }
        out
    }

    #[test]
    fn parses_a_well_formed_cube_file() {
        let lut = Lut3D::parse(&identity_cube(4)).unwrap();
        assert_eq!(lut.size, 4);
        assert_eq!(lut.data.len(), 64);
    }

    #[test]
    fn ignores_comments_blank_lines_and_metadata() {
        let mut text = String::from("# a comment\nTITLE \"test\"\n\n");
        text.push_str(&identity_cube(2));
        text.push_str("DOMAIN_MIN 0.0 0.0 0.0\nDOMAIN_MAX 1.0 1.0 1.0\n");
        let lut = Lut3D::parse(&text).unwrap();
        assert_eq!(lut.size, 2);
    }

    #[test]
    fn missing_size_is_an_error() {
        assert_eq!(
            Lut3D::parse("0.0 0.0 0.0\n"),
            Err(LutParseError::MissingSize)
        );
    }

    #[test]
    fn size_too_small_is_an_error() {
        assert_eq!(
            Lut3D::parse("LUT_3D_SIZE 1\n0 0 0\n"),
            Err(LutParseError::SizeTooSmall(1))
        );
    }

    #[test]
    fn wrong_row_count_is_an_error() {
        assert_eq!(
            Lut3D::parse("LUT_3D_SIZE 2\n0 0 0\n1 1 1\n"),
            Err(LutParseError::WrongRowCount {
                expected: 8,
                found: 2
            })
        );
    }

    #[test]
    fn identity_lut_samples_are_a_no_op() {
        let lut = Lut3D::parse(&identity_cube(16)).unwrap();
        let [r, g, b] = lut.sample(0.5, 0.25, 0.75);
        assert!((r - 0.5).abs() < 0.05);
        assert!((g - 0.25).abs() < 0.05);
        assert!((b - 0.75).abs() < 0.05);
    }

    #[test]
    fn identity_lut_corners_are_exact() {
        let lut = Lut3D::parse(&identity_cube(4)).unwrap();
        assert_eq!(lut.sample(0.0, 0.0, 0.0), [0.0, 0.0, 0.0]);
        assert_eq!(lut.sample(1.0, 1.0, 1.0), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn a_lut_that_zeroes_red_darkens_only_the_red_channel_of_the_frame() {
        // Every entry maps to (0, g, b) -- an identity LUT for green/blue, but red always
        // zeroed regardless of the input red value. On a 2x2x2 grid, identity-mapped g/b still
        // interpolate back to (approximately) their original continuous input value -- this
        // isn't "the original pixel with red zeroed" by construction, it's what the interpolated
        // LUT lookup happens to reproduce for a linear identity mapping.
        let size = 2;
        let mut text = format!("LUT_3D_SIZE {size}\n");
        for b in 0..size {
            for g in 0..size {
                for _r in 0..size {
                    text.push_str(&format!("0.0 {:.1} {:.1}\n", g as f32, b as f32));
                }
            }
        }
        let lut = Lut3D::parse(&text).unwrap();
        let mut rgba = vec![255u8, 128, 64, 200];
        apply_lut_to_rgba(&mut rgba, &lut);
        assert_eq!(rgba[0], 0, "red zeroed regardless of input");
        assert!(
            (rgba[1] as i32 - 128).abs() <= 1,
            "green preserved via identity"
        );
        assert!(
            (rgba[2] as i32 - 64).abs() <= 1,
            "blue preserved via identity"
        );
        assert_eq!(rgba[3], 200, "alpha untouched");
    }

    #[test]
    fn vignette_leaves_the_center_pixel_unchanged() {
        let width = 4;
        let height = 4;
        let mut rgba = vec![200u8; (width * height * 4) as usize];
        apply_vignette_to_rgba(&mut rgba, width, height, 1.0);
        // The four center pixels of a 4x4 grid are the closest to dead-center.
        let center_index = ((1 * width + 1) * 4) as usize;
        assert!(
            rgba[center_index] > 150,
            "near-center pixel barely darkened"
        );
    }

    #[test]
    fn vignette_darkens_corners_more_than_the_center() {
        let width = 20;
        let height = 20;
        let mut rgba = vec![255u8; (width * height * 4) as usize];
        apply_vignette_to_rgba(&mut rgba, width, height, 0.8);
        let center_index = ((10 * width + 10) * 4) as usize;
        let corner_index = 0usize;
        assert!(
            rgba[corner_index] < rgba[center_index],
            "corner ({}) should be darker than center ({})",
            rgba[corner_index],
            rgba[center_index]
        );
    }

    #[test]
    fn zero_intensity_vignette_is_a_no_op() {
        let mut rgba = vec![100u8; 4 * 4 * 4];
        let before = rgba.clone();
        apply_vignette_to_rgba(&mut rgba, 4, 4, 0.0);
        assert_eq!(rgba, before);
    }

    #[test]
    fn vignette_leaves_alpha_untouched() {
        let mut rgba = vec![0u8, 0, 0, 77];
        apply_vignette_to_rgba(&mut rgba, 1, 1, 1.0);
        assert_eq!(rgba[3], 77);
    }
}
