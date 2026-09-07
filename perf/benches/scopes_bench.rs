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

//! Benchmarks `avcore::scopes`'s waveform/vectorscope monitors at 1920x1080 — the resolution
//! `App::pump_preview_frame` renders both from on every decoded preview frame whenever
//! `scopes_enabled`. Compares calling `luma_waveform_rgba`/`vectorscope_rgba` separately (two
//! full-frame passes, the old call site's own shape) against the combined `render_scopes_rgba`
//! (one pass). See REPORT.md's scopes section for the measured numbers.

use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oca_perf::scopes::{luma_waveform_rgba, render_scopes_rgba, vectorscope_rgba};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const WAVEFORM_OUT: (u32, u32) = (256, 128);
const VECTORSCOPE_OUT: u32 = 128;

fn frame() -> Vec<u8> {
    (0..(WIDTH as usize * HEIGHT as usize))
        .flat_map(|i| {
            let x = (i % WIDTH as usize) as u32;
            let y = (i / WIDTH as usize) as u32;
            [
                ((x * 37 + y * 19) % 256) as u8,
                ((x * 11 + y * 53) % 256) as u8,
                ((x * 71 + y * 5) % 256) as u8,
                255,
            ]
        })
        .collect()
}

fn bench_scopes(c: &mut Criterion) {
    let src = frame();
    let mut group = c.benchmark_group("scopes (1920x1080)");
    group.bench_function(BenchmarkId::new("before", "two_separate_passes"), |b| {
        b.iter(|| {
            let waveform = luma_waveform_rgba(
                black_box(&src),
                WIDTH,
                HEIGHT,
                WAVEFORM_OUT.0,
                WAVEFORM_OUT.1,
            );
            let vectorscope = vectorscope_rgba(black_box(&src), WIDTH, HEIGHT, VECTORSCOPE_OUT);
            black_box((waveform, vectorscope))
        })
    });
    group.bench_function(BenchmarkId::new("after", "render_scopes_rgba"), |b| {
        b.iter(|| {
            black_box(render_scopes_rgba(
                black_box(&src),
                WIDTH,
                HEIGHT,
                WAVEFORM_OUT.0,
                WAVEFORM_OUT.1,
                VECTORSCOPE_OUT,
            ))
        })
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .sample_size(200);
    targets = bench_scopes
}
criterion_main!(benches);
