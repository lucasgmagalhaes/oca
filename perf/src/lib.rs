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

//! Measurement harness for `oca`'s per-frame CPU-side preview effects
//! (`crates/core/src/preview_effects.rs`) — see `PLAN.md`/`REPORT.md` in this directory for the
//! full methodology and results.
//!
//! **Why a standalone crate copy, not `#[path]`-including the real file directly**: this crate
//! must build and run in a sandboxed environment that cannot link `core` at all (no network
//! access to fetch `libonnxruntime`, `core`'s `ort` dependency needs it even to link a test/bench
//! harness binary, confirmed repeatedly in this repository's own session notes — see
//! `CLAUDE.md`). `preview_effects.rs` itself has zero heavy dependencies (already confirmed
//! elsewhere in this repo's history via the same technique), so a byte-for-byte functional copy
//! here is fully linkable and runnable for real, at the cost of needing to be kept in sync by
//! hand with the real source if either changes. Each function below names the real source
//! function it mirrors.
//!
//! Each hot function that received a real fix in this pass (`apply_lut_to_rgba`,
//! `apply_glitch_to_rgba`, `apply_deflicker_to_rgba`) is duplicated here as a `_before`/`_after`
//! pair — `_before` is the original `chunks_exact`/`chunks_exact_mut` implementation (still
//! reflects what the code looked like prior to this pass), `_after` is the `as_chunks`/
//! `as_chunks_mut` version now actually committed to `crates/core/src/preview_effects.rs`. The
//! benchmarks in `benches/preview_effects_bench.rs` run both so the improvement is a measured
//! number, not an assumption.

/// A parsed 3D LUT — mirrors `avcore::preview_effects::Lut3D`.
pub struct Lut3D {
    size: usize,
    data: Vec<[f32; 3]>,
}

impl Lut3D {
    /// Mirrors `Lut3D::parse` exactly (error variants collapsed to `Result<Self, ()>` — this
    /// harness only needs "did parsing succeed," never the specific reason).
    pub fn parse(contents: &str) -> Result<Self, ()> {
        let mut size: Option<usize> = None;
        let mut data = Vec::new();

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("LUT_3D_SIZE") {
                let n: i64 = rest.trim().parse().map_err(|_| ())?;
                if n < 2 {
                    return Err(());
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
                return Err(());
            }
            data.push([values[0], values[1], values[2]]);
        }

        let size = size.ok_or(())?;
        let expected = size * size * size;
        if data.len() != expected {
            return Err(());
        }
        Ok(Self { size, data })
    }

    fn at(&self, x: usize, y: usize, z: usize) -> [f32; 3] {
        self.data[x + y * self.size + z * self.size * self.size]
    }

    /// Mirrors `Lut3D::sample` exactly.
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

/// Builds an identity LUT of the given size — a `.cube` document string, same shape the real
/// `preview_effects.rs` test module's own `identity_cube` helper builds, for benchmark input.
pub fn identity_cube(size: usize) -> String {
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

/// `apply_lut_to_rgba`, **before** this pass's fix — `chunks_exact_mut(4)` (a runtime-computed
/// chunk size, plus a runtime remainder check per call even though the size is always 4).
pub fn apply_lut_to_rgba_before(rgba: &mut [u8], lut: &Lut3D) {
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

/// `apply_lut_to_rgba`, **after** — `as_chunks_mut::<4>()` (the chunk size is a const generic,
/// known at compile time; `clippy::chunks_exact_to_as_chunks` flagged the `_before` form).
pub fn apply_lut_to_rgba_after(rgba: &mut [u8], lut: &Lut3D) {
    for pixel in rgba.as_chunks_mut::<4>().0 {
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

/// `apply_vignette_to_rgba` — mirrors the real function exactly. Not part of this pass's fix
/// (already manual index arithmetic, not `chunks_exact`), benchmarked anyway to characterize its
/// cost alongside the other three per-frame effects.
pub fn apply_vignette_to_rgba(rgba: &mut [u8], width: u32, height: u32, intensity: f32) {
    if width == 0 || height == 0 || intensity <= 0.0 {
        return;
    }
    if rgba.len() != (width as usize) * (height as usize) * 4 {
        return;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
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

/// `apply_glitch_to_rgba`, **before**.
pub fn apply_glitch_to_rgba_before(rgba: &mut [u8], intensity: f32, seed: u64) {
    let intensity = intensity.clamp(0.0, 1.0);
    if intensity <= 0.0 || rgba.is_empty() {
        return;
    }
    let strength = (intensity * 40.0).round() as i64;
    if strength <= 0 {
        return;
    }
    let span = (2 * strength + 1) as u64;
    for (pixel_index, chunk) in rgba.chunks_exact_mut(4).enumerate() {
        let mut state = seed ^ (pixel_index as u64).wrapping_mul(0x9E3779B97F4A7C15);
        for channel in chunk.iter_mut().take(3) {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let noise = (state % span) as i64 - strength;
            *channel = (*channel as i64 + noise).clamp(0, 255) as u8;
        }
    }
}

/// `apply_glitch_to_rgba`, **after**.
pub fn apply_glitch_to_rgba_after(rgba: &mut [u8], intensity: f32, seed: u64) {
    let intensity = intensity.clamp(0.0, 1.0);
    if intensity <= 0.0 || rgba.is_empty() {
        return;
    }
    let strength = (intensity * 40.0).round() as i64;
    if strength <= 0 {
        return;
    }
    let span = (2 * strength + 1) as u64;
    for (pixel_index, chunk) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let mut state = seed ^ (pixel_index as u64).wrapping_mul(0x9E3779B97F4A7C15);
        for channel in chunk.iter_mut().take(3) {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let noise = (state % span) as i64 - strength;
            *channel = (*channel as i64 + noise).clamp(0, 255) as u8;
        }
    }
}

#[derive(Default)]
pub struct DeflickerHistory {
    means: std::collections::VecDeque<f32>,
}

impl DeflickerHistory {
    const WINDOW: usize = 5;

    pub fn new() -> Self {
        Self::default()
    }
}

/// `apply_deflicker_to_rgba`, **before**.
pub fn apply_deflicker_to_rgba_before(rgba: &mut [u8], history: &mut DeflickerHistory) {
    if rgba.is_empty() || rgba.len() % 4 != 0 {
        return;
    }

    let pixel_count = rgba.len() / 4;
    let mut luma_sum = 0.0_f64;
    for chunk in rgba.chunks_exact(4) {
        luma_sum += 0.299 * chunk[0] as f64 + 0.587 * chunk[1] as f64 + 0.114 * chunk[2] as f64;
    }
    let current_mean = (luma_sum / pixel_count as f64) as f32;

    history.means.push_back(current_mean);
    if history.means.len() > DeflickerHistory::WINDOW {
        history.means.pop_front();
    }
    let rolling_average = history.means.iter().sum::<f32>() / history.means.len() as f32;

    let shift = (rolling_average - current_mean).round() as i32;
    if shift == 0 {
        return;
    }
    for chunk in rgba.chunks_exact_mut(4) {
        for channel in chunk.iter_mut().take(3) {
            *channel = (*channel as i32 + shift).clamp(0, 255) as u8;
        }
    }
}

/// `apply_deflicker_to_rgba`, **after**.
pub fn apply_deflicker_to_rgba_after(rgba: &mut [u8], history: &mut DeflickerHistory) {
    if rgba.is_empty() || !rgba.len().is_multiple_of(4) {
        return;
    }

    let pixel_count = rgba.len() / 4;
    let mut luma_sum = 0.0_f64;
    for chunk in rgba.as_chunks::<4>().0 {
        luma_sum += 0.299 * chunk[0] as f64 + 0.587 * chunk[1] as f64 + 0.114 * chunk[2] as f64;
    }
    let current_mean = (luma_sum / pixel_count as f64) as f32;

    history.means.push_back(current_mean);
    if history.means.len() > DeflickerHistory::WINDOW {
        history.means.pop_front();
    }
    let rolling_average = history.means.iter().sum::<f32>() / history.means.len() as f32;

    let shift = (rolling_average - current_mean).round() as i32;
    if shift == 0 {
        return;
    }
    for chunk in rgba.as_chunks_mut::<4>().0 {
        for channel in chunk.iter_mut().take(3) {
            *channel = (*channel as i32 + shift).clamp(0, 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms the `_before`/`_after` pair are behaviorally identical for every function, on a
    /// mixed-content buffer — the fix must be a pure refactor, never a behavior change. Run for
    /// real (`cargo test`), not just type-checked, since this crate has no linking gap.
    fn sample_rgba() -> Vec<u8> {
        (0..(16 * 16 * 4)).map(|i| (i % 256) as u8).collect()
    }

    #[test]
    fn lut_before_and_after_agree() {
        let lut = Lut3D::parse(&identity_cube(8)).unwrap();
        let mut a = sample_rgba();
        let mut b = a.clone();
        apply_lut_to_rgba_before(&mut a, &lut);
        apply_lut_to_rgba_after(&mut b, &lut);
        assert_eq!(a, b);
    }

    #[test]
    fn glitch_before_and_after_agree() {
        let mut a = sample_rgba();
        let mut b = a.clone();
        apply_glitch_to_rgba_before(&mut a, 0.7, 42);
        apply_glitch_to_rgba_after(&mut b, 0.7, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn deflicker_before_and_after_agree() {
        let mut a = sample_rgba();
        let mut b = a.clone();
        let mut hist_a = DeflickerHistory::new();
        let mut hist_b = DeflickerHistory::new();
        // Seed both histories identically across a few frames before the frame under test.
        for _ in 0..3 {
            let mut dark_a = vec![50u8; 16 * 16 * 4];
            let mut dark_b = dark_a.clone();
            apply_deflicker_to_rgba_before(&mut dark_a, &mut hist_a);
            apply_deflicker_to_rgba_after(&mut dark_b, &mut hist_b);
        }
        apply_deflicker_to_rgba_before(&mut a, &mut hist_a);
        apply_deflicker_to_rgba_after(&mut b, &mut hist_b);
        assert_eq!(a, b);
    }
}
