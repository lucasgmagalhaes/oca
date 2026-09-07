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

//! Mirrors `avcore::scopes` (`crates/core/src/scopes.rs`) — see this crate's own top-level doc
//! comment for why this is a functional copy, not an import. `luma_waveform_rgba`/
//! `vectorscope_rgba` are the two pre-existing standalone functions (each does its own full pass
//! over the source frame); `render_scopes_rgba` is the new combined-pass function that computes
//! both in one iteration over the source pixels.

fn luma(r: u8, g: u8, b: u8) -> u8 {
    (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64).round() as u8
}

fn chroma_cb_cr(r: u8, g: u8, b: u8) -> (u8, u8) {
    let (r, g, b) = (r as f64, g as f64, b as f64);
    let cb = (-0.168736 * r - 0.331264 * g + 0.5 * b + 128.0).clamp(0.0, 255.0);
    let cr = (0.5 * r - 0.418688 * g - 0.081312 * b + 128.0).clamp(0.0, 255.0);
    (cb.round() as u8, cr.round() as u8)
}

pub fn luma_waveform_rgba(
    src_rgba: &[u8],
    src_w: u32,
    src_h: u32,
    out_w: u32,
    out_h: u32,
) -> Vec<u8> {
    if src_w == 0 || src_h == 0 || out_w == 0 || out_h == 0 {
        return Vec::new();
    }
    const GAIN: f64 = 6.0;
    let mut counts = vec![0u32; (out_w * out_h) as usize];
    for y in 0..src_h {
        for x in 0..src_w {
            let i = ((y * src_w + x) * 4) as usize;
            let l = luma(src_rgba[i], src_rgba[i + 1], src_rgba[i + 2]);
            let col = (x * out_w / src_w).min(out_w - 1);
            let row = (out_h - 1).saturating_sub((l as u32 * (out_h - 1)) / 255);
            counts[(row * out_w + col) as usize] += 1;
        }
    }
    let column_total = src_h as f64;
    let mut rgba = vec![0u8; (out_w * out_h * 4) as usize];
    for (i, &count) in counts.iter().enumerate() {
        let intensity = ((count as f64 / column_total) * GAIN * 255.0).clamp(0.0, 255.0) as u8;
        let px = i * 4;
        rgba[px] = intensity;
        rgba[px + 1] = intensity;
        rgba[px + 2] = intensity;
        rgba[px + 3] = 255;
    }
    rgba
}

pub fn vectorscope_rgba(src_rgba: &[u8], src_w: u32, src_h: u32, out_size: u32) -> Vec<u8> {
    if src_w == 0 || src_h == 0 || out_size == 0 {
        return Vec::new();
    }
    const GAIN: f64 = 40.0;
    let mut counts = vec![0u32; (out_size * out_size) as usize];
    for y in 0..src_h {
        for x in 0..src_w {
            let i = ((y * src_w + x) * 4) as usize;
            let (cb, cr) = chroma_cb_cr(src_rgba[i], src_rgba[i + 1], src_rgba[i + 2]);
            let gx = ((cb as u32) * (out_size - 1).max(1)) / 255;
            let gy = (out_size - 1).saturating_sub(((cr as u32) * (out_size - 1).max(1)) / 255);
            let gx = gx.min(out_size - 1);
            let gy = gy.min(out_size - 1);
            counts[(gy * out_size + gx) as usize] += 1;
        }
    }
    let total_pixels = (src_w as u64 * src_h as u64) as f64;
    let mut rgba = vec![0u8; (out_size * out_size * 4) as usize];
    for (i, &count) in counts.iter().enumerate() {
        let intensity = ((count as f64 / total_pixels) * GAIN * 255.0).clamp(0.0, 255.0) as u8;
        let px = i * 4;
        rgba[px] = intensity;
        rgba[px + 1] = intensity;
        rgba[px + 2] = intensity;
        rgba[px + 3] = 255;
    }
    rgba
}

pub fn render_scopes_rgba(
    src_rgba: &[u8],
    src_w: u32,
    src_h: u32,
    waveform_out_w: u32,
    waveform_out_h: u32,
    vectorscope_out_size: u32,
) -> (Vec<u8>, Vec<u8>) {
    if src_w == 0 || src_h == 0 {
        return (Vec::new(), Vec::new());
    }
    let waveform_ready = waveform_out_w != 0 && waveform_out_h != 0;
    let vectorscope_ready = vectorscope_out_size != 0;

    let mut waveform_counts = vec![0u32; (waveform_out_w * waveform_out_h) as usize];
    let mut vectorscope_counts = vec![0u32; (vectorscope_out_size * vectorscope_out_size) as usize];

    for y in 0..src_h {
        for x in 0..src_w {
            let i = ((y * src_w + x) * 4) as usize;
            let (r, g, b) = (src_rgba[i], src_rgba[i + 1], src_rgba[i + 2]);
            if waveform_ready {
                let l = luma(r, g, b);
                let col = (x * waveform_out_w / src_w).min(waveform_out_w - 1);
                let row =
                    (waveform_out_h - 1).saturating_sub((l as u32 * (waveform_out_h - 1)) / 255);
                waveform_counts[(row * waveform_out_w + col) as usize] += 1;
            }
            if vectorscope_ready {
                let (cb, cr) = chroma_cb_cr(r, g, b);
                let gx = ((cb as u32) * (vectorscope_out_size - 1).max(1)) / 255;
                let gy = (vectorscope_out_size - 1)
                    .saturating_sub(((cr as u32) * (vectorscope_out_size - 1).max(1)) / 255);
                let gx = gx.min(vectorscope_out_size - 1);
                let gy = gy.min(vectorscope_out_size - 1);
                vectorscope_counts[(gy * vectorscope_out_size + gx) as usize] += 1;
            }
        }
    }

    let waveform_rgba = if waveform_ready {
        const GAIN: f64 = 6.0;
        let column_total = src_h as f64;
        let mut rgba = vec![0u8; (waveform_out_w * waveform_out_h * 4) as usize];
        for (i, &count) in waveform_counts.iter().enumerate() {
            let intensity = ((count as f64 / column_total) * GAIN * 255.0).clamp(0.0, 255.0) as u8;
            let px = i * 4;
            rgba[px] = intensity;
            rgba[px + 1] = intensity;
            rgba[px + 2] = intensity;
            rgba[px + 3] = 255;
        }
        rgba
    } else {
        Vec::new()
    };

    let vectorscope_rgba = if vectorscope_ready {
        const GAIN: f64 = 40.0;
        let total_pixels = (src_w as u64 * src_h as u64) as f64;
        let mut rgba = vec![0u8; (vectorscope_out_size * vectorscope_out_size * 4) as usize];
        for (i, &count) in vectorscope_counts.iter().enumerate() {
            let intensity = ((count as f64 / total_pixels) * GAIN * 255.0).clamp(0.0, 255.0) as u8;
            let px = i * 4;
            rgba[px] = intensity;
            rgba[px + 1] = intensity;
            rgba[px + 2] = intensity;
            rgba[px + 3] = 255;
        }
        rgba
    } else {
        Vec::new()
    };

    (waveform_rgba, vectorscope_rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varied_rgba(width: u32, height: u32) -> Vec<u8> {
        let mut buf = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let r = ((x * 37 + y * 19) % 256) as u8;
                let g = ((x * 11 + y * 53) % 256) as u8;
                let b = ((x * 71 + y * 5) % 256) as u8;
                buf.extend_from_slice(&[r, g, b, 255]);
            }
        }
        buf
    }

    #[test]
    fn render_scopes_matches_calling_both_standalone_functions() {
        let src = varied_rgba(37, 23);
        let (combined_waveform, combined_vectorscope) =
            render_scopes_rgba(&src, 37, 23, 17, 11, 13);
        assert_eq!(combined_waveform, luma_waveform_rgba(&src, 37, 23, 17, 11));
        assert_eq!(combined_vectorscope, vectorscope_rgba(&src, 37, 23, 13));
    }
}
